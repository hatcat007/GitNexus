#!/bin/sh
set -u

echo "[startup] $(date -Iseconds) memvid-export-api container boot"
echo "[startup] uid=$(id -u) gid=$(id -g) pwd=$(pwd)"
echo "[startup] bind_addr=${MEMVID_EXPORT_BIND_ADDR:-<unset>}"
echo "[startup] export_root=${MEMVID_EXPORT_ROOT:-<unset>}"
echo "[startup] retention_seconds=${MEMVID_EXPORT_RETENTION_SECONDS:-<unset>}"

if [ -n "${MEMVID_EXPORT_API_KEY:-}" ]; then
  echo "[startup] api_key=present length=${#MEMVID_EXPORT_API_KEY}"
else
  echo "[startup] api_key=missing"
fi

if [ -x /usr/local/bin/memvid-export-api ]; then
  echo "[startup] binary=/usr/local/bin/memvid-export-api executable=yes"
  ls -l /usr/local/bin/memvid-export-api || true
else
  echo "[startup] binary=/usr/local/bin/memvid-export-api executable=no"
fi

if [ -n "${MEMVID_EXPORT_ROOT:-}" ]; then
  if [ -d "${MEMVID_EXPORT_ROOT}" ]; then
    echo "[startup] export_root_exists=yes"
    ls -ld "${MEMVID_EXPORT_ROOT}" || true
  else
    echo "[startup] export_root_exists=no"
  fi
fi

echo "[startup] launching memvid-export-api ..."
set +e
/usr/local/bin/memvid-export-api
exit_code=$?
set -e
echo "[startup] memvid-export-api exited with code=${exit_code}"

sleep_seconds="${MEMVID_DEBUG_EXIT_SLEEP_SECONDS:-5}"
echo "[startup] sleeping ${sleep_seconds}s before exit for log visibility"
sleep "${sleep_seconds}"
exit "${exit_code}"
