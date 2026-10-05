#!/usr/bin/env bash

# Loads one explicitly supplied PostgreSQL URL for host-side test harnesses.
# The value is printed only to the caller's command substitution. Errors never
# include the configured URL or file contents.
revenant_database_url() {
  if [[ -n "${DATABASE_URL:-}" && -n "${DATABASE_URL_FILE:-}" ]]; then
    echo "DATABASE_URL and DATABASE_URL_FILE cannot both be configured" >&2
    return 1
  fi

  local value
  if [[ -n "${DATABASE_URL:-}" ]]; then
    value="$DATABASE_URL"
  elif [[ -n "${DATABASE_URL_FILE:-}" ]]; then
    if [[ ! -f "$DATABASE_URL_FILE" || ! -r "$DATABASE_URL_FILE" ]]; then
      echo "DATABASE_URL_FILE is not a readable regular file" >&2
      return 1
    fi
    IFS= read -r value < "$DATABASE_URL_FILE" || [[ -n "$value" ]]
  else
    echo "DATABASE_URL or DATABASE_URL_FILE is required" >&2
    return 1
  fi

  if [[ ${#value} -lt 1 || ${#value} -gt 2048 || "$value" == *$'\n'* ||
        "$value" == *$'\r'* ]] ||
     [[ "$value" != postgres://* && "$value" != postgresql://* ]]; then
    echo "database URL configuration has an invalid shape" >&2
    return 1
  fi
  printf '%s' "$value"
}
