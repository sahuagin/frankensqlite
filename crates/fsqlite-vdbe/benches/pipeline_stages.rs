use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use fsqlite_pager::{
    MvccPager, SimplePager, SimpleTransaction, TransactionHandle, TransactionMode,
};
use fsqlite_types::opcode::{Opcode, P4};
use fsqlite_types::record::{parse_record, serialize_record};
use fsqlite_types::value::SqliteValue;
use fsqlite_types::{Cx, PageNumber, PageSize};
use fsqlite_vdbe::engine::{VdbeEngine, set_vdbe_jit_enabled};
use fsqlite_vdbe::{
    ProgramBuilder, VdbeProgram, profile_vdbe_commit_stage, profile_vdbe_decode_stage,
};
use fsqlite_vfs::MemoryVfs;
use std::path::Path;

const EXECUTE_STAGE_OP_REPEATS: [usize; 3] = [64, 256, 1024];
const COMMIT_STAGE_DIRTY_PAGES: [usize; 3] = [2, 8, 32];

fn decode_stage_row(column_count: usize) -> Vec<SqliteValue> {
    (0..column_count)
        .map(|idx| match idx % 3 {
            0 => SqliteValue::Integer(i64::try_from(idx * 97 + 11).unwrap()),
            1 => SqliteValue::Text(format!("decode-stage-{idx:03}").into()),
            _ => SqliteValue::Blob(vec![u8::try_from((idx % 251) + 1).unwrap(); 24].into()),
        })
        .collect()
}

fn build_execute_stage_program(op_repeats: usize) -> VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let end = builder.emit_label();
    builder.emit_jump_to_label(Opcode::Init, 0, 0, end, P4::None, 0);
    let accumulator = builder.alloc_reg();
    builder.emit_op(Opcode::Integer, 0, accumulator, 0, P4::None, 0);
    for _ in 0..op_repeats {
        builder.emit_op(Opcode::AddImm, accumulator, 1, 0, P4::None, 0);
    }
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);
    builder.resolve_label(end);
    builder
        .finish()
        .expect("pipeline execute benchmark program should build")
}

/// Build a dispatch-dominated program whose inner loop is a stream of
/// single-register `Copy` ops. The source register holds an `Integer`, so
/// the body reduces to `clone + set_reg_fast` per dispatch — the work is
/// small enough that the hot-path pre-filter vs main-match routing is the
/// dominant cost, which is exactly the effect we want to measure.
fn build_execute_stage_copy_program(op_repeats: usize) -> VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let end = builder.emit_label();
    builder.emit_jump_to_label(Opcode::Init, 0, 0, end, P4::None, 0);
    let src = builder.alloc_reg();
    let dst = builder.alloc_reg();
    builder.emit_op(Opcode::Integer, 42, src, 0, P4::None, 0);
    for _ in 0..op_repeats {
        // p1=src, p2=dst, p3=0 (copy a single register)
        builder.emit_op(Opcode::Copy, src, dst, 0, P4::None, 0);
    }
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);
    builder.resolve_label(end);
    builder
        .finish()
        .expect("pipeline execute copy benchmark program should build")
}

/// Build a dispatch-dominated program whose inner loop is a stream of
/// single-register `SCopy` (shallow-copy) ops. Like the Copy variant, the
/// source holds an `Integer`, so the body is `clone + set_reg_fast` per
/// dispatch — isolating the hot-path pre-filter vs main-match routing
/// cost for the SCopy arm specifically.
fn build_execute_stage_scopy_program(op_repeats: usize) -> VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let end = builder.emit_label();
    builder.emit_jump_to_label(Opcode::Init, 0, 0, end, P4::None, 0);
    let src = builder.alloc_reg();
    let dst = builder.alloc_reg();
    builder.emit_op(Opcode::Integer, 42, src, 0, P4::None, 0);
    for _ in 0..op_repeats {
        builder.emit_op(Opcode::SCopy, src, dst, 0, P4::None, 0);
    }
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);
    builder.resolve_label(end);
    builder
        .finish()
        .expect("pipeline execute scopy benchmark program should build")
}

/// Build a dispatch-dominated program whose inner loop is a stream of
/// `DecrJumpZero` ops (the canonical LIMIT counter opcode). The counter
/// is seeded with `op_repeats + 1` so every dispatched opcode hits the
/// decrement-and-fall-through path — none jump to the halt target,
/// giving a stable per-op cost that isolates the hot-path pre-filter
/// routing from the mostly-taken branch predictor.
fn build_execute_stage_decrjumpzero_program(op_repeats: usize) -> VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let end = builder.emit_label();
    let halt = builder.emit_label();
    builder.emit_jump_to_label(Opcode::Init, 0, 0, end, P4::None, 0);
    let counter = builder.alloc_reg();
    // Seed so op_repeats decrements leave the counter at 1 (never zero,
    // so the jump is never taken).
    let seed = i32::try_from(op_repeats + 1).unwrap_or(i32::MAX);
    builder.emit_op(Opcode::Integer, seed, counter, 0, P4::None, 0);
    for _ in 0..op_repeats {
        builder.emit_jump_to_label(Opcode::DecrJumpZero, counter, 0, halt, P4::None, 0);
    }
    builder.resolve_label(halt);
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);
    builder.resolve_label(end);
    builder
        .finish()
        .expect("pipeline execute decrjumpzero benchmark program should build")
}

/// Build a dispatch-dominated program whose inner loop is a stream of
/// `IfPos` ops (the canonical OFFSET counter opcode). Each op's p2 jump
/// target is the instruction immediately after it, so a "jump" is
/// semantically equivalent to a fall-through for execution sequencing
/// but still exercises the opcode's taken-branch body (register read,
/// subtract, write-back, pc reassignment). p3=1 makes each op decrement
/// the counter by one; counter is seeded with `op_repeats + 1` so the
/// val>0 branch is taken every iteration.
fn build_execute_stage_ifpos_program(op_repeats: usize) -> VdbeProgram {
    let mut builder = ProgramBuilder::new();
    let end = builder.emit_label();
    builder.emit_jump_to_label(Opcode::Init, 0, 0, end, P4::None, 0);
    let counter = builder.alloc_reg();
    let seed = i32::try_from(op_repeats + 1).unwrap_or(i32::MAX);
    builder.emit_op(Opcode::Integer, seed, counter, 0, P4::None, 0);
    for _ in 0..op_repeats {
        let next = builder.emit_label();
        // p1=counter, p3=1 (decrement by one), p2=next (the very next
        // instruction — so this is an always-taken, fall-through-style jump).
        builder.emit_jump_to_label(Opcode::IfPos, counter, 1, next, P4::None, 0);
        builder.resolve_label(next);
    }
    builder.emit_op(Opcode::Halt, 0, 0, 0, P4::None, 0);
    builder.resolve_label(end);
    builder
        .finish()
        .expect("pipeline execute ifpos benchmark program should build")
}

fn prepare_commit_stage_fixture(dirty_pages: usize) -> (Cx, SimpleTransaction<MemoryVfs>) {
    let cx = Cx::new();
    let pager = SimplePager::open_with_cx(
        &cx,
        MemoryVfs::new(),
        Path::new("/:memory:"),
        PageSize::DEFAULT,
    )
    .expect("pipeline commit benchmark should open pager");
    let mut txn = pager
        .begin(&cx, TransactionMode::Immediate)
        .expect("pipeline commit benchmark should begin transaction");
    let page_bytes = PageSize::DEFAULT.as_usize();
    txn.write_page(&cx, PageNumber::ONE, &vec![0xA5; page_bytes])
        .expect("pipeline commit benchmark should dirty page one");
    for page_idx in 1..dirty_pages {
        let page_no = txn
            .allocate_page(&cx)
            .expect("pipeline commit benchmark should allocate page");
        let fill = u8::try_from((page_idx % 251) + 1).unwrap();
        txn.write_page(&cx, page_no, &vec![fill; page_bytes])
            .expect("pipeline commit benchmark should dirty page");
    }
    (cx, txn)
}

fn bench_vdbe_decode_stage(c: &mut Criterion) {
    let mut group = c.benchmark_group("vdbe_pipeline_decode");

    for column_count in [8_usize, 32, 128] {
        let record = serialize_record(&decode_stage_row(column_count));
        group.throughput(Throughput::Bytes(
            u64::try_from(record.len()).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(column_count),
            &record,
            |b, record| {
                b.iter(|| {
                    let decoded = profile_vdbe_decode_stage(|| {
                        parse_record(black_box(record.as_slice()))
                            .expect("pipeline decode benchmark should parse record")
                    });
                    black_box(decoded);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_execute_stage(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("vdbe_pipeline_execute");

    for op_repeats in EXECUTE_STAGE_OP_REPEATS {
        let program = build_execute_stage_program(op_repeats);
        group.throughput(Throughput::Elements(
            u64::try_from(op_repeats).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(op_repeats),
            &program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                engine.set_collect_result_rows(false);
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("pipeline execute benchmark should execute");
                    black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_execute_copy_stage(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("vdbe_pipeline_execute_copy");

    for op_repeats in EXECUTE_STAGE_OP_REPEATS {
        let program = build_execute_stage_copy_program(op_repeats);
        group.throughput(Throughput::Elements(
            u64::try_from(op_repeats).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(op_repeats),
            &program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                engine.set_collect_result_rows(false);
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("pipeline execute copy benchmark should execute");
                    black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_execute_scopy_stage(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("vdbe_pipeline_execute_scopy");

    for op_repeats in EXECUTE_STAGE_OP_REPEATS {
        let program = build_execute_stage_scopy_program(op_repeats);
        group.throughput(Throughput::Elements(
            u64::try_from(op_repeats).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(op_repeats),
            &program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                engine.set_collect_result_rows(false);
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("pipeline execute scopy benchmark should execute");
                    black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_execute_decrjumpzero_stage(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("vdbe_pipeline_execute_decrjumpzero");

    for op_repeats in EXECUTE_STAGE_OP_REPEATS {
        let program = build_execute_stage_decrjumpzero_program(op_repeats);
        group.throughput(Throughput::Elements(
            u64::try_from(op_repeats).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(op_repeats),
            &program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                engine.set_collect_result_rows(false);
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("pipeline execute decrjumpzero benchmark should execute");
                    black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_execute_ifpos_stage(c: &mut Criterion) {
    set_vdbe_jit_enabled(false);
    let mut group = c.benchmark_group("vdbe_pipeline_execute_ifpos");

    for op_repeats in EXECUTE_STAGE_OP_REPEATS {
        let program = build_execute_stage_ifpos_program(op_repeats);
        group.throughput(Throughput::Elements(
            u64::try_from(op_repeats).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(op_repeats),
            &program,
            |b, program| {
                let execution_cx = Cx::new();
                let mut engine = VdbeEngine::new_with_execution_cx(
                    program.register_count(),
                    &execution_cx,
                    PageSize::DEFAULT,
                );
                engine.set_collect_result_rows(false);
                b.iter(|| {
                    let outcome = engine
                        .execute(program)
                        .expect("pipeline execute ifpos benchmark should execute");
                    black_box(outcome);
                });
            },
        );
    }

    group.finish();
}

fn bench_vdbe_commit_stage(c: &mut Criterion) {
    let mut group = c.benchmark_group("vdbe_pipeline_commit");

    for dirty_pages in COMMIT_STAGE_DIRTY_PAGES {
        group.throughput(Throughput::Elements(
            u64::try_from(dirty_pages).unwrap_or(u64::MAX),
        ));
        group.bench_with_input(
            BenchmarkId::from_parameter(dirty_pages),
            &dirty_pages,
            |b, &dirty_pages| {
                b.iter_batched(
                    || prepare_commit_stage_fixture(dirty_pages),
                    |(cx, mut txn)| {
                        profile_vdbe_commit_stage(|| {
                            txn.commit(&cx)
                                .expect("pipeline commit benchmark should commit");
                        });
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_vdbe_decode_stage,
    bench_vdbe_execute_stage,
    bench_vdbe_execute_copy_stage,
    bench_vdbe_execute_scopy_stage,
    bench_vdbe_execute_decrjumpzero_stage,
    bench_vdbe_commit_stage
);
criterion_main!(benches);
