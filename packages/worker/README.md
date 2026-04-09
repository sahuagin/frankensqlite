# `@frankensqlite/worker`

`@frankensqlite/worker` owns the browser worker-side transport for
FrankenSQLite's WebAssembly bindings. It loads `@frankensqlite/core`,
maintains a single worker-owned `FrankenDB` instance, and exposes a typed
message protocol that higher-level SDKs can consume.

Current behavior:

- `memory` persistence works end to end.
- `opfs` and `indexeddb` are rejected with explicit "not implemented yet"
  worker errors instead of silently falling back.

## Package Surface

- `createFrankenSqliteWorker()` creates a module worker from the packaged
  `worker.js` entrypoint.
- `WorkerConnectionHost` handles request/response dispatch inside the worker.
- `protocol` exports the request/response/message types shared with the SDK.

## Example

```ts
import { createFrankenSqliteWorker } from "@frankensqlite/worker";

const worker = createFrankenSqliteWorker();
worker.postMessage({
  kind: "init",
  requestId: 1,
  config: { persistence: "memory" },
});
```
