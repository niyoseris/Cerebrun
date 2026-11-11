#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

mkdir -p backups
STAMP=$(date +%Y%m%d-%H%M%S)
OUT="backups/cerebrun-${STAMP}.dump"

docker compose exec -T db sh -c 'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Fc' > "$OUT"

echo "Backup written: $OUT"
