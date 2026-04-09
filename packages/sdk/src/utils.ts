import { createFrankenSqliteWorker } from "@frankensqlite/worker";

import type { FrankenDbOpenOptions } from "./types";
import type { WorkerLike } from "./worker-client";

export function normalizeOpenOptions(
  options?: FrankenDbOpenOptions | string,
): FrankenDbOpenOptions {
  if (typeof options === "string") {
    return { dbName: options };
  }
  return {
    persistence: "memory",
    ...options,
  };
}

export function resolveWorker(
  worker: FrankenDbOpenOptions["worker"],
): WorkerLike {
  if (typeof worker === "function") {
    return worker();
  }
  if (worker) {
    return worker;
  }
  return createFrankenSqliteWorker();
}
