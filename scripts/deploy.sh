#!/usr/bin/env bash
# Deploy deskrouter. Credentials come from AWS_PROFILE or the AWS credential
# chain (e.g. instance metadata); never from files or arguments.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
pattern='AKIA[0-9A-Z]{16}|ASIA[0-9A-Z]{16}|wJal[A-Za-z0-9/+=]{10,}'

if grep -rEq "$pattern" "$root/.claude/skills" 2>/dev/null; then
  echo "error: credential-shaped string found in skill docs" >&2
  exit 1
fi
if printf '%s\n' "$*" | grep -Eq "$pattern"; then
  echo "error: credential-shaped string found in arguments" >&2
  exit 1
fi
if [ -n "${AWS_ACCESS_KEY_ID:-}${AWS_SECRET_ACCESS_KEY:-}" ]; then
  echo "error: static AWS keys in environment; use AWS_PROFILE or the credential chain" >&2
  exit 1
fi

echo "Deploying with profile: ${AWS_PROFILE:-<default credential chain>}"
# TODO: invoke the actual deployment tooling here.
