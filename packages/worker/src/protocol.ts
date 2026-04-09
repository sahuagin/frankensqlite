export type PersistenceMode = "memory" | "opfs" | "indexeddb";

export type SqlScalar =
  | null
  | string
  | number
  | bigint
  | boolean
  | Uint8Array;

export type SqlParams = SqlScalar[];

export interface QueryResult<Row extends Record<string, unknown> = Record<string, unknown>> {
  columns: string[];
  columnCount: number;
  columnTypes: string[];
  rows: Row[];
  rowArrays: SqlScalar[][];
  changes: number;
}

export interface InitConfig {
  dbName?: string;
  persistence?: PersistenceMode;
  wasmUrl?: string;
  snapshot?: Uint8Array;
}

export interface InitResult {
  path: string;
  persistence: PersistenceMode;
}

export interface PreparedStatementMetadata {
  statementId: string;
  sql: string;
  columnCount: number;
  columnNames: string[];
}

export interface SerializedFrankenError {
  code: string;
  message: string;
  sqliteCode?: number;
  extendedCode?: number;
  transient?: boolean;
  userRecoverable?: boolean;
  suggestion?: string;
  stack?: string;
}

interface WorkerRequestBase {
  requestId: number;
}

interface WorkerResponseBase {
  requestId: number;
}

export interface InitRequest extends WorkerRequestBase {
  kind: "init";
  config: InitConfig;
}

export interface ExecuteRequest extends WorkerRequestBase {
  kind: "execute";
  sql: string;
  params?: SqlParams;
}

export interface ExecuteBatchRequest extends WorkerRequestBase {
  kind: "execute-batch";
  sql: string;
}

export interface QueryRequest extends WorkerRequestBase {
  kind: "query";
  sql: string;
  params?: SqlParams;
}

export interface PrepareRequest extends WorkerRequestBase {
  kind: "prepare";
  sql: string;
}

export interface StatementExecuteRequest extends WorkerRequestBase {
  kind: "statement-execute";
  statementId: string;
  params?: SqlParams;
}

export interface StatementQueryRequest extends WorkerRequestBase {
  kind: "statement-query";
  statementId: string;
  params?: SqlParams;
}

export interface StatementFinalizeRequest extends WorkerRequestBase {
  kind: "statement-finalize";
  statementId: string;
}

export interface ExportRequest extends WorkerRequestBase {
  kind: "export";
}

export interface CloseRequest extends WorkerRequestBase {
  kind: "close";
}

export type WorkerRequest =
  | InitRequest
  | ExecuteRequest
  | ExecuteBatchRequest
  | QueryRequest
  | PrepareRequest
  | StatementExecuteRequest
  | StatementQueryRequest
  | StatementFinalizeRequest
  | ExportRequest
  | CloseRequest;

export interface ReadyResponse extends WorkerResponseBase {
  kind: "ready";
  data: InitResult;
}

export interface ExecuteResponse extends WorkerResponseBase {
  kind: "execute-result";
  changes: number;
}

export interface ExecuteBatchResponse extends WorkerResponseBase {
  kind: "execute-batch-result";
}

export interface QueryResponse extends WorkerResponseBase {
  kind: "query-result";
  data: QueryResult;
}

export interface PrepareResponse extends WorkerResponseBase {
  kind: "prepare-result";
  data: PreparedStatementMetadata;
}

export interface StatementFinalizeResponse extends WorkerResponseBase {
  kind: "statement-finalize-result";
}

export interface ExportResponse extends WorkerResponseBase {
  kind: "export-result";
  data: Uint8Array;
}

export interface CloseResponse extends WorkerResponseBase {
  kind: "close-result";
}

export interface ErrorResponse extends WorkerResponseBase {
  kind: "error";
  error: SerializedFrankenError;
}

export type WorkerResponse =
  | ReadyResponse
  | ExecuteResponse
  | ExecuteBatchResponse
  | QueryResponse
  | PrepareResponse
  | StatementFinalizeResponse
  | ExportResponse
  | CloseResponse
  | ErrorResponse;
