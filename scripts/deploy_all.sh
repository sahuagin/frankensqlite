#!/bin/bash
set -euo pipefail

SITE_DIR="site/spec-evolution"

echo "Starting deployment sequence..."
mkdir -p dist
mkdir -p dist/data
cp "$SITE_DIR/visualization_of_the_evolution_of_the_frankensqlite_specs_document_from_inception.html" dist/index.html
cp "$SITE_DIR/data/spec_evolution_v1.sqlite3" dist/data/
cp "$SITE_DIR/data/spec_evolution_v1.sqlite3.config.json" dist/data/
cp "$SITE_DIR/og-image.png" dist/
cp "$SITE_DIR/twitter-image.png" dist/
cp "$SITE_DIR/frankensqlite_illustration.webp" dist/
cp "$SITE_DIR/frankensqlite_diagram.webp" dist/
cp "$SITE_DIR/_headers" dist/
cp "$SITE_DIR/_routes.json" dist/
npx wrangler pages deploy dist --project-name frankensqlite-spec-evolution --branch main --commit-dirty=true
echo "Deployment sequence completed."
