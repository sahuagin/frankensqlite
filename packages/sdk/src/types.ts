import type {
  InitConfig,
  PersistenceMode,
  QueryResult as WorkerQueryResult,
  SerializedFrankenError,
  SqlScalar,
} from "@frankensqlite/worker";

import type { WorkerLike } from "./worker-client";

export type { PersistenceMode, SerializedFrankenError, SqlScalar };

export type QueryResult<Row extends Record<string, unknown> = Record<string, unknown>> =
  WorkerQueryResult<Row>;

export interface FrankenDbOpenOptions
  extends Omit<InitConfig, "snapshot"> {
  snapshot?: Uint8Array;
  worker?: WorkerLike | (() => WorkerLike);
}
