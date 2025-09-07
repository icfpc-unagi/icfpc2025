#!/usr/bin/env bash
set -Eeuo pipefail

# Runs the target command, killing and retrying if it exceeds 5 minutes.
# Stops entirely if the command exits successfully.

# Config
TIME_LIMIT="5m"          # Changeable via env before calling this script
KILL_GRACE="10s"         # Grace period before SIGKILL after timeout

# Command + input
CMD=(cargo run --release --bin wata_sat3)
INPUT="remote zain"

# Ensure `timeout` exists
if ! command -v timeout >/dev/null 2>&1; then
  echo "error: GNU coreutils 'timeout' command not found" >&2
  echo "Install coreutils or adjust this script to use a manual timeout." >&2
  exit 127
fi

echo "[run_repeated] starting loop at $(date)"
echo "[run_repeated] time limit: ${TIME_LIMIT} (grace ${KILL_GRACE})"

while true; do
  ./run unlock -f
  echo "[run_repeated] launching at $(date)"
  # Use a subshell via bash -lc so we can use here-string and 'time'.
  # timeout exit codes: 124 on timeout, otherwise child's exit status.
  if timeout \
      --signal=TERM \
      --kill-after="${KILL_GRACE}" \
      "${TIME_LIMIT}" \
      bash -lc "time ${CMD[*]} <<< \"${INPUT}\""; then
    echo "[run_repeated] finished successfully at $(date)"
    exit 0
  else
    status=$?
    # Retry on any non-zero status (including 124/137/143 timeouts).
    echo "[run_repeated] exited with status ${status}; retrying..." >&2
    continue
  fi
done
