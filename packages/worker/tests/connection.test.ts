import { describe, expect, it } from "vitest";

import { WorkerConnectionHost } from "../src/connection";
import type {
  CoreDatabaseConstructor,
  CoreDatabaseHandle,
  CoreModuleLoader,
  CorePreparedStatementHandle,
} from "../src/connection";
import type { QueryResult } from "../src/protocol";

class FakeStatement implements CorePreparedStatementHandle {
  readonly sql: string;
  readonly columnCount: number;
  readonly #rows: QueryResult;

  constructor(sql: string, rows: QueryResult) {
    this.sql = sql;
    this.columnCount = rows.columnCount;
    this.#rows = rows;
  }

  columnNames(): string[] {
    return [...this.#rows.columns];
  }

  execute(): number {
    return 1;
  }

  executeWithParams(): number {
    return 1;
  }

  query(): QueryResult {
    return this.#rows;
  }

  queryWithParams(): QueryResult {
    return this.#rows;
  }
}

class FakeDatabase implements CoreDatabaseHandle {
  readonly path: string;

  constructor(path = ":memory:") {
    this.path = path;
  }

  close(): void {}

  execute(): number {
    return 1;
  }

  executeBatch(): void {}

  executeWithParams(): number {
    return 1;
  }

  query(): QueryResult {
    return {
      columns: ["id", "name"],
      columnCount: 2,
      columnTypes: ["integer", "text"],
      rows: [{ id: 1, name: "alpha" }],
      rowArrays: [[1, "alpha"]],
      changes: 0,
    };
  }

  queryWithParams(): QueryResult {
    return this.query();
  }

  prepare(sql: string): CorePreparedStatementHandle {
    return new FakeStatement(sql, this.query());
  }

  export(): Uint8Array {
    return Uint8Array.of(1, 2, 3, 4);
  }

  static import(): CoreDatabaseHandle {
    return new FakeDatabase(":memory:");
  }
}

const fakeLoader: CoreModuleLoader = {
  async load() {
    return {
      FrankenDB: FakeDatabase as unknown as CoreDatabaseConstructor,
    };
  },
};

describe("WorkerConnectionHost", () => {
  it("initializes a memory database and returns ready metadata", async () => {
    const host = new WorkerConnectionHost(fakeLoader);
    const response = await host.handle({
      kind: "init",
      requestId: 1,
      config: { dbName: "demo", persistence: "memory" },
    });

    expect(response.kind).toBe("ready");
    if (response.kind === "ready") {
      expect(response.data.persistence).toBe("memory");
      expect(response.data.path).toBe("demo");
    }
  });

  it("rejects persistence modes that are not implemented yet", async () => {
    const host = new WorkerConnectionHost(fakeLoader);
    const response = await host.handle({
      kind: "init",
      requestId: 1,
      config: { dbName: "demo", persistence: "opfs" },
    });

    expect(response.kind).toBe("error");
    if (response.kind === "error") {
      expect(response.error.code).toBe("ERR_FSQLITE_UNSUPPORTED_PERSISTENCE");
      expect(response.error.message).toContain("not implemented yet");
    }
  });

  it("supports prepare, query, export, and close lifecycle requests", async () => {
    const host = new WorkerConnectionHost(fakeLoader);
    await host.handle({
      kind: "init",
      requestId: 1,
      config: { persistence: "memory" },
    });

    const prepared = await host.handle({
      kind: "prepare",
      requestId: 2,
      sql: "SELECT id, name FROM demo",
    });
    expect(prepared.kind).toBe("prepare-result");
    if (prepared.kind !== "prepare-result") {
      return;
    }

    const queried = await host.handle({
      kind: "statement-query",
      requestId: 3,
      statementId: prepared.data.statementId,
    });
    expect(queried.kind).toBe("query-result");
    if (queried.kind === "query-result") {
      expect(queried.data.rows).toEqual([{ id: 1, name: "alpha" }]);
    }

    const exported = await host.handle({
      kind: "export",
      requestId: 4,
    });
    expect(exported.kind).toBe("export-result");
    if (exported.kind === "export-result") {
      expect([...exported.data]).toEqual([1, 2, 3, 4]);
    }

    const finalized = await host.handle({
      kind: "statement-finalize",
      requestId: 5,
      statementId: prepared.data.statementId,
    });
    expect(finalized.kind).toBe("statement-finalize-result");

    const closed = await host.handle({
      kind: "close",
      requestId: 6,
    });
    expect(closed.kind).toBe("close-result");
  });
});
