import { FrankenPreparedStatement } from "./statement";
import { FrankenTransaction } from "./transaction";
import type { FrankenDbOpenOptions, QueryResult } from "./types";
import { normalizeOpenOptions, resolveWorker } from "./utils";
import { FrankenWorkerClient } from "./worker-client";

export class FrankenDB {
  readonly #client: FrankenWorkerClient;
  readonly #path: string;

  private constructor(client: FrankenWorkerClient, path: string) {
    this.#client = client;
    this.#path = path;
  }

  static async open(options?: FrankenDbOpenOptions | string): Promise<FrankenDB> {
    const normalized = normalizeOpenOptions(options);
    const client = new FrankenWorkerClient(resolveWorker(normalized.worker));
    const ready = await client.init({
      dbName: normalized.dbName,
      persistence: normalized.persistence,
      wasmUrl: normalized.wasmUrl,
      snapshot: normalized.snapshot,
    });
    return new FrankenDB(client, ready.path);
  }

  static import(
    snapshot: Uint8Array,
    options?: Omit<FrankenDbOpenOptions, "snapshot">,
  ): Promise<FrankenDB> {
    return FrankenDB.open({
      ...options,
      snapshot,
    });
  }

  get path(): string {
    return this.#path;
  }

  execute(sql: string, params: readonly unknown[] = []): Promise<number> {
    return this.#client.execute(sql, params);
  }

  executeBatch(sql: string): Promise<void> {
    return this.#client.executeBatch(sql);
  }

  query<Row extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
    params: readonly unknown[] = [],
  ): Promise<QueryResult<Row>> {
    return this.#client.query(sql, params);
  }

  async prepare<Row extends Record<string, unknown> = Record<string, unknown>>(
    sql: string,
  ): Promise<FrankenPreparedStatement<Row>> {
    const metadata = await this.#client.prepare(sql);
    return new FrankenPreparedStatement<Row>(
      this.#client,
      metadata.statementId,
      metadata.sql,
      metadata.columnCount,
      metadata.columnNames,
    );
  }

  export(): Promise<Uint8Array> {
    return this.#client.export();
  }

  async transaction<T>(
    work: (tx: FrankenTransaction) => Promise<T>,
  ): Promise<T> {
    await this.executeBatch("BEGIN");
    try {
      const result = await work(new FrankenTransaction(this));
      await this.executeBatch("COMMIT");
      return result;
    } catch (error: unknown) {
      await this.executeBatch("ROLLBACK");
      throw error;
    }
  }

  async close(): Promise<void> {
    try {
      await this.#client.close();
    } finally {
      this.#client.dispose();
    }
  }
}
