# Spec Evolution Site Assets

This directory contains the source assets for the static spec-evolution
visualization deployed by the root `package.json` build script and
`scripts/deploy*.sh`.

The `data/spec_evolution_v1.sqlite3` database is intentionally tracked even
though SQLite files are normally treated as local scratch data in this
repository. The browser page loads it directly through `sql.js`, and it is
deterministically regenerated from `data/spec_evolution_data_v1.json.gz` by
`tools/generate-spec-evolution-sqlite.py`. The adjacent
`data/spec_evolution_v1.sqlite3.config.json` records the cache-busting hash used
by the deploy health checks.

Cloudflare Pages headers and routes are also source-controlled here as
`_headers` and `_routes.json`. The deploy scripts copy them into `dist/`; root
copies are ignored so generated deployment files do not clutter the project
root again.
