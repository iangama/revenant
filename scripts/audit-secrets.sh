#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

mapfile -d '' candidates < <(git ls-files --cached --others --exclude-standard -z)
for path in "${candidates[@]}"; do
  case "/$path" in
    */.env|*.pem|*.p12|*.pfx|*.key)
      echo "secret audit rejected sensitive path: $path" >&2
      exit 1
      ;;
  esac
done

if ((${#candidates[@]} > 0)); then
  if rg --line-number --with-filename \
    --glob '!*.lock' \
    --glob '!.env.example' \
    --glob '!scripts/audit-secrets.sh' \
    "(-----BEGIN ([A-Z ]+ )?PRIVATE KEY-----|AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{30,}|github_pat_[A-Za-z0-9_]{40,}|(?i:(api[_-]?key|secret|token|password))[[:space:]]*[:=][[:space:]]*[\"'][A-Za-z0-9+/=_-]{16,})" \
    "${candidates[@]}"; then
    echo "secret audit found a credential-like value" >&2
    exit 1
  fi
fi

echo "secret audit passed (${#candidates[@]} candidate files)"
