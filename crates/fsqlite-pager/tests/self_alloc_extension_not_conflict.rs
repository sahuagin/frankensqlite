use std::path::Path;

use fsqlite_pager::traits::{JournalMode, MvccPager, TransactionHandle, TransactionMode};
use fsqlite_pager::SimplePager;
use fsqlite_types::cx::Cx;
use fsqlite_types::PageSize;
use fsqlite_vfs::MemoryVfs;

#[test]
fn self_allocated_eof_page_stays_out_of_conflict_surface() {
    let cx = Cx::new();
    let pager = SimplePager::open_with_cx(
        &cx,
        MemoryVfs::new(),
        Path::new("/self_alloc_extension.db"),
        PageSize::DEFAULT,
    )
    .expect("pager should open");
    pager
        .set_journal_mode(&cx, JournalMode::Wal)
        .expect("WAL mode should be available");

    let mut txn = pager
        .begin(&cx, TransactionMode::Concurrent)
        .expect("concurrent transaction should begin");
    let page = txn.allocate_page(&cx).expect("allocation should extend EOF");
    assert_eq!(
        page.get(),
        2,
        "fresh database should extend from page 1 to page 2"
    );
    txn.write_page(&cx, page, &[0xA5; 64])
        .expect("newly allocated page should accept writes");

    let pending_commit = txn
        .pending_commit_pages()
        .expect("pending commit surface should be available");
    let pending_conflict = txn
        .pending_conflict_pages()
        .expect("pending conflict surface should be available");

    assert!(
        pending_commit.contains(&page),
        "self-allocated extension page must be written at commit"
    );
    assert!(
        !pending_conflict.contains(&page),
        "self-allocated EOF extension page {page:?} must not be treated as a cross-process conflict page; pending_conflict={pending_conflict:?}"
    );
}
