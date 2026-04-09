# `@frankensqlite/sdk`

`@frankensqlite/sdk` provides the async, worker-backed TypeScript client for
FrankenSQLite in browser environments.

Current behavior:

- `FrankenDB.open()` starts a dedicated module worker and initializes the WASM
  runtime through `@frankensqlite/worker`.
- `execute`, `executeBatch`, `query`, `prepare`, `export`, and `transaction`
  are exposed as Promise-based APIs.
- Persistence is intentionally memory-first until OPFS and IndexedDB backends
  land. Passing `opfs` or `indexeddb` surfaces an explicit worker error.

## Example

```ts
import { FrankenDB } from "@frankensqlite/sdk";

const db = await FrankenDB.open({ persistence: "memory" });
await db.execute("CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT)");
await db.execute("INSERT INTO users(name) VALUES (?)", ["Ada"]);

const result = await db.query<{ id: number; name: string }>(
  "SELECT id, name FROM users ORDER BY id",
);

console.log(result.rows);
await db.close();
```
