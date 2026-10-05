#!/usr/bin/env bash
set -euo pipefail
archive_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 -B "$archive_root/operator/verify.py" "$archive_root"
export REVENANT_GAME_HOST=127.0.0.1
export REVENANT_GAME_PORT="${REVENANT_GAME_PORT:-7000}"
export REVENANT_PLAYTEST_MODE=0
exec "$archive_root/client/linux/Revenant.x86_64" "$@"
