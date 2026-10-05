#!/usr/bin/env bash
set -euo pipefail
archive_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project="${COMPOSE_PROJECT_NAME:-infra}"
export REVENANT_SECRETS_DIR="${REVENANT_SECRETS_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/revenant/m30-secrets/current}"
compose=(docker compose -p "$project" -f "$archive_root/backend/compose.json")
case "${1:-status}" in
  start)
    python3 -B "$archive_root/operator/verify.py" "$archive_root"
    if [[ ! -d "$REVENANT_SECRETS_DIR" ]]; then
      if docker volume inspect "${project}_revenant-postgres" >/dev/null 2>&1; then
        echo 'Existing saves require their existing private credentials. Set REVENANT_SECRETS_DIR.' >&2
        exit 1
      fi
      REVENANT_SECRETS_ROOT="$(dirname "$REVENANT_SECRETS_DIR")" \
        bash "$archive_root/backend/m30-generate-secrets.sh" "$(basename "$REVENANT_SECRETS_DIR")"
    fi
    docker load --input "$archive_root/backend/images.tar"
    "${compose[@]}" up -d --no-build --pull never --wait --wait-timeout 90
    ;;
  status) "${compose[@]}" ps ;;
  stop) "${compose[@]}" stop ;;
  *) echo 'Usage: bash operator/stack.sh start|status|stop' >&2; exit 2 ;;
esac
