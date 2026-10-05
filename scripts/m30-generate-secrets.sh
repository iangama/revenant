#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
account_home="$(getent passwd "$(id -u)" | cut -d: -f6)"
if [[ -z "$account_home" || "$account_home" != /* ]]; then
  echo "could not resolve the current account home directory" >&2
  exit 1
fi
state_root="${XDG_STATE_HOME:-$account_home/.local/state}"
secret_root="${REVENANT_SECRETS_ROOT:-$state_root/revenant/m30-secrets}"
generation="${1:-current}"

if [[ ! "$generation" =~ ^[a-z0-9][a-z0-9-]{0,31}$ ]]; then
  echo "secret generation name must contain 1-32 lowercase letters, digits, or hyphens" >&2
  exit 1
fi
if [[ ! "$secret_root" = /* ]]; then
  echo "REVENANT_SECRETS_ROOT must be an absolute path" >&2
  exit 1
fi

target="$secret_root/$generation"
if [[ -e "$target" ]]; then
  echo "refusing to overwrite an existing secret generation: $target" >&2
  exit 1
fi

install -d -m 700 "$secret_root"
temporary="$(mktemp -d "$secret_root/.candidate-XXXXXX")"
cleanup() {
  if [[ -d "$temporary" ]]; then
    find "$temporary" -maxdepth 1 -type f -exec sh -c 'printf "" > "$1"' _ {} \;
    rm -rf "$temporary"
  fi
}
trap cleanup EXIT
chmod 700 "$temporary"

generate_value() {
  openssl rand -base64 32 | tr '+/' '-_' | tr -d '=\r\n'
}

admin_password="$(generate_value)"
runtime_password="$(generate_value)"
if [[ ! "$admin_password" =~ ^[A-Za-z0-9_-]{43}$ ]] ||
   [[ ! "$runtime_password" =~ ^[A-Za-z0-9_-]{43}$ ]] ||
   [[ "$admin_password" == "$runtime_password" ]]; then
  echo "operating-system secret generation returned an invalid shape" >&2
  exit 1
fi

umask 077
printf '%s\n' "$admin_password" > "$temporary/postgres_admin_password"
printf '%s\n' "$runtime_password" > "$temporary/postgres_runtime_password"
printf 'postgres://revenant:%s@postgres:5432/revenant\n' "$admin_password" \
  > "$temporary/postgres_admin_database_url"
printf 'postgres://revenant_runtime:%s@postgres:5432/revenant\n' "$runtime_password" \
  > "$temporary/gateway_database_url"
printf 'postgres://revenant_runtime:%s@127.0.0.1:5432/revenant\n' "$runtime_password" \
  > "$temporary/operator_database_url"
unset admin_password runtime_password

while IFS= read -r path; do
  chmod 600 "$path"
done < <(find "$temporary" -maxdepth 1 -type f -print)
permissions_valid=yes
while IFS= read -r path; do
  if [[ "$(stat -c '%a' "$path")" != "600" ]]; then
    permissions_valid=no
  fi
done < <(find "$temporary" -maxdepth 1 -type f -print)
if [[ "$(stat -c '%a' "$temporary")" != "700" || "$permissions_valid" != yes ]]; then
  echo "secret filesystem did not enforce owner-only permissions" >&2
  exit 1
fi
mv "$temporary" "$target"
trap - EXIT

printf 'secret_generation=%s\nfiles=5\ndirectory_mode=0700\nfile_mode=0600\n' "$target"
