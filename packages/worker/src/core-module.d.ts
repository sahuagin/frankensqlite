declare module "@frankensqlite/core" {
  export interface CoreQueryResult<Row extends Record<string, unknown> = Record<string, unknown>> {
    columns: string[];
    columnCount: number;
    columnTypes: string[];
    rows: Row[];
    rowArrays: unknown[][];
    changes: number;
  }

  export class FrankenPreparedStatement {
    readonly sql: string;
    readonly columnCount: number;
    columnNames(): string[];
    execute(): number;
    executeWithParams(params: unknown[]): number;
    query(): CoreQueryResult;
    queryWithParams(params: unknown[]): CoreQueryResult;
    explain(): string;
  }

  export class FrankenDB {
    constructor(name?: string);
    static open(name?: string): FrankenDB;
    static import(data: Uint8Array): FrankenDB;
    readonly path: string;
    close(): void;
    execute(sql: string): number;
    executeBatch(sql: string): void;
    executeWithParams(sql: string, params: unknown[]): number;
    query(sql: string): CoreQueryResult;
    queryWithParams(sql: string, params: unknown[]): CoreQueryResult;
    prepare(sql: string): FrankenPreparedStatement;
    export(): Uint8Array;
    explain(sql: string): string;
  }

  export default function init(
    input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module,
  ): Promise<void>;
}
