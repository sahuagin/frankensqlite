# FrankenSQLite Contracts

This directory contains durable, machine-readable project contracts that are
consumed by verification scripts, harness tests, and release/parity reports.
They intentionally live outside the repository root so generated scratch files,
local SQLite databases, benchmark reports, and one-off agent artifacts do not
crowd the project entrypoint.

Keep new contract-style TOML artifacts here unless a toolchain convention
requires a root-level file.
