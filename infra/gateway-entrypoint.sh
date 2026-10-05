#!/bin/sh
set -eu

runtime_directory=/run/revenant
runtime_url_file="$runtime_directory/database_url"

if [ "$(id -u)" -ne 0 ]; then
  echo "gateway entrypoint requires root only for secret staging and privilege drop" >&2
  exit 1
fi
if [ -z "${DATABASE_URL_FILE:-}" ] || [ ! -f "$DATABASE_URL_FILE" ] ||
   [ ! -r "$DATABASE_URL_FILE" ]; then
  echo "gateway database secret file is unavailable" >&2
  exit 1
fi

mkdir -p "$runtime_directory"
umask 077
cp "$DATABASE_URL_FILE" "$runtime_url_file"
chown revenant:revenant "$runtime_url_file"
chmod 0400 "$runtime_url_file"
export DATABASE_URL_FILE="$runtime_url_file"

exec su-exec revenant:revenant /revenant-gateway "$@"
