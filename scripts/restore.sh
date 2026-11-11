#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

FILE="${1:-}"
if [ -z "$FILE" ] || [ ! -f "$FILE" ]; then
  echo "usage: $0 backups/<file>.dump" >&2
  exit 1
fi

# Always bring the app back, even if the restore fails.
trap 'docker compose start app >/dev/null 2>&1 || true' EXIT

echo "Stopping app..."
docker compose stop app >/dev/null

echo "Restoring $FILE (this replaces the current data)..."
cat "$FILE" | docker compose exec -T db sh -c \
  'pg_restore -U "$POSTGRES_USER" -d "$POSTGRES_DB" --clean --if-exists --no-owner'

echo "Starting app..."
docker compose start app >/dev/null
echo "Restore complete."
