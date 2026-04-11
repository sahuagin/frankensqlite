# fsqlite-cli

Interactive SQL shell for FrankenSQLite, providing a REPL and single-command
execution mode.

## Overview

`fsqlite-cli` is the command-line binary for FrankenSQLite. It compiles to the
`fsqlite` executable and provides:

- **Interactive REPL** -- Multi-line SQL input with TTY-aware prompts. The
  shell keeps classic `fsqlite> ` / `...> ` prompts for `:memory:` sessions and
  shows the current database name after `.open` or when launched against a file.
- **TTY syntax highlighting** -- Continuation prompts render a highlighted
  preview of the pending SQL statement so interactive sessions surface keywords,
  literals, and comments more clearly.
- **Single-command mode** (`-c` / `--command`) -- Execute one SQL string and
  exit. SQL and supported dot-commands both work here, which makes the CLI
  easier to script.
- **sqlite3-style dot commands** -- `.read`, `.open`, `.tables`, `.schema`,
  `.dump`, `.mode`, and `.headers` / `.header` are built in for common shell
  workflows.
- **Batch mode for piped stdin** -- When stdin/stdout are not attached to a TTY,
  prompts are suppressed automatically so pipelines stay clean, and `-batch` /
  `--batch` can force the same behavior on a TTY.
- **Decode proof verification** (`--verify-proof`) -- Verify ECS decode proofs
  from a JSON file, with configurable policy ID and slack parameters.
- **In-memory or file-backed** -- Defaults to `:memory:` if no database path is
  provided; pass a file path as a positional argument for persistent storage.

**Position in the dependency graph:**

```
fsqlite-cli (this crate) -- the user-facing binary
  --> fsqlite (public API facade)
    --> fsqlite-core (engine)
      --> fsqlite-parser, fsqlite-planner, fsqlite-vdbe, ...
```

Dependencies: `fsqlite`, `fsqlite-core`, `fsqlite-error`, `fsqlite-types`,
`serde`, `serde_json`.

## Key Types

- `CliOptions` -- Parsed command-line arguments (db path, command, verify-proof
  path, policy ID, slack, help flag).
- `main()` / `run()` -- Entry point. Dispatches to REPL, single-command
  execution, or proof verification based on CLI arguments.

## Usage

```bash
# Interactive REPL with in-memory database
fsqlite

# Interactive REPL with a file-backed database
fsqlite mydb.sqlite

# Execute a single SQL command
fsqlite mydb.sqlite -c "SELECT * FROM users;"

# Execute against in-memory database
fsqlite -c "SELECT 1 + 2;"

# Run dot commands in command mode
fsqlite demo.db -c ".schema users"
fsqlite demo.db -c ".dump"

# Feed SQL through stdin without prompts in the output
printf 'SELECT 1;\n' | fsqlite demo.db

# Verify an ECS decode proof
fsqlite --verify-proof proof.json

# Show help
fsqlite --help
```

### CLI Options

| Flag | Description |
|------|-------------|
| `<db_path>` | Database file path (default: `:memory:`) |
| `-c`, `--command <SQL>` | Execute SQL or a dot-command and exit |
| `-batch`, `--batch` | Force batch-mode execution with prompts disabled |
| `-init`, `--init <FILE>` | Run a startup script before command mode or the REPL |
| `--verify-proof <path>` | Verify ECS decode proof from JSON file |
| `--verify-policy-id <N>` | Policy ID for proof verification |
| `--verify-slack <N>` | Slack parameter for proof verification |
| `-h`, `--help` | Show usage information |

### Dot Commands

| Command | Description |
|---------|-------------|
| `.help` | Show shell help |
| `.open <path>` | Re-open the shell against another database |
| `.tables [pattern]` | List tables and views, optionally filtered by `LIKE` pattern |
| `.schema [pattern]` | Print schema SQL, optionally filtered by `LIKE` pattern |
| `.dump [pattern]` | Emit schema and table contents as SQL text |
| `.mode <mode>` | Set output mode: `list`, `column`, `csv`, `tabs`, `line` |
| `.headers <on\|off>` | Toggle column headers (`.header` alias also works) |
| `.read <path>` | Execute commands from a file |
| `.quit`, `.exit` | Leave the shell |

## License

MIT (with OpenAI/Anthropic Rider) -- see workspace root LICENSE file.
