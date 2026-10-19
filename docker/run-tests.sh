#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE=(docker compose -f "$ROOT/docker/docker-compose.yml")
CLUSTER_FILE="$ROOT/docker/fdb.cluster"
CLUSTER_CONTENTS="docker:docker@127.0.0.1:4500"
TEST_ARGS=(-- tests --nocapture --test-threads=1)

preflight() {
  command -v docker >/dev/null 2>&1 || { echo "error: docker not found in PATH" >&2; exit 1; }
  docker compose version >/dev/null 2>&1 || { echo "error: docker compose plugin not available" >&2; exit 1; }
  have_libfdb_c() {
    # no grep -q here: with pipefail, -q's early exit can SIGPIPE ldconfig and fail the pipeline
    ldconfig -p 2>/dev/null | grep libfdb_c >/dev/null && return 0
    ls /usr/lib*/libfdb_c.so* /lib*/libfdb_c.so* /usr/lib/*/libfdb_c.so* >/dev/null 2>&1
  }
  have_libfdb_c || { echo "error: libfdb_c not found; install foundationdb-clients 7.3.79" >&2; exit 1; }
}

check_port() {
  # already our own container? then the bound port is expected, not a conflict
  if "${COMPOSE[@]}" ps --status running -q fdb 2>/dev/null | grep . >/dev/null; then
    return 0
  fi
  if ss -ltn 2>/dev/null | awk '{print $4}' | grep ':4500$' >/dev/null; then
    echo "warning: host port 4500 is already in use; docker may fail to bind it" >&2
    echo "warning: stop any host fdbserver (e.g. 'sudo systemctl stop foundationdb')" >&2
  fi
}

wait_for_server() {
  echo "waiting for FDB server..."
  local out=""
  for _ in $(seq 1 30); do
    out=$("${COMPOSE[@]}" exec -T fdb fdbcli --exec "status minimal" 2>&1 || true)
    if echo "$out" | grep -qE "database is (available|unavailable)"; then
      return 0
    fi
    sleep 1
  done
  echo "error: FDB server did not become reachable" >&2
  echo "$out" >&2
  exit 1
}

configure_db() {
  local out=""
  for _ in $(seq 1 30); do
    out=$("${COMPOSE[@]}" exec -T fdb fdbcli --exec "configure new single memory" 2>&1 || true)
    if echo "$out" | grep -qiE "Database created|already"; then
      return 0
    fi
    sleep 1
  done
  echo "error: failed to configure FDB database" >&2
  echo "$out" >&2
  exit 1
}

write_cluster_file() {
  mkdir -p "$(dirname "$CLUSTER_FILE")"
  echo "$CLUSTER_CONTENTS" > "$CLUSTER_FILE"
}

cmd_up() {
  preflight
  check_port
  "${COMPOSE[@]}" up -d
  wait_for_server
  configure_db
  write_cluster_file
  echo "FDB ready (cluster file: $CLUSTER_FILE)"
}

cmd_test() {
  cmd_up
  cd "$ROOT"
  FDB_CLUSTER_FILE="$CLUSTER_FILE" cargo test "${TEST_ARGS[@]}"
}

cmd_down() { "${COMPOSE[@]}" down; }
cmd_reset() { "${COMPOSE[@]}" down -v; cmd_up; }
cmd_logs() { "${COMPOSE[@]}" logs -f; }

case "${1:-test}" in
  test)  cmd_test ;;
  up)    cmd_up ;;
  down)  cmd_down ;;
  reset) cmd_reset ;;
  logs)  cmd_logs ;;
  *) echo "usage: $0 [test|up|down|reset|logs]" >&2; exit 1 ;;
esac
