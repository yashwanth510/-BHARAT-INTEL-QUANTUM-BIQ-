#!/usr/bin/env bash
set -euo pipefail
BIQ_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$BIQ_ROOT"
mkdir -p .runtime
case "${1:-start}" in
  start)
    test -f .env || { echo 'Copy .env.example to .env and configure Redis first.'; exit 1; }
    for service in backend frontend; do
      if [[ -f ".runtime/$service.pid" ]] && kill -0 "$(cat ".runtime/$service.pid")" 2>/dev/null; then
        echo "$service already running"
        continue
      fi
      if [[ "$service" == backend ]]; then
        test -x backend/target/debug/biq-backend || cargo build --locked --manifest-path backend/Cargo.toml
        setsid nohup backend/target/debug/biq-backend > .runtime/backend.log 2>&1 < /dev/null &
      else
        test -d frontend/node_modules || npm --prefix frontend ci
        (cd frontend && exec setsid nohup node node_modules/next/dist/bin/next dev --hostname 0.0.0.0 --port 3000) > .runtime/frontend.log 2>&1 < /dev/null &
      fi
      echo "$!" > ".runtime/$service.pid"
    done
    for service in backend frontend; do
      sleep 1
      if ! kill -0 "$(cat ".runtime/$service.pid")" 2>/dev/null; then
        echo "$service failed to start; inspect .runtime/$service.log" >&2
        exit 1
      fi
    done
    echo 'Frontend: http://localhost:3000 | Backend: http://localhost:8000/health'
    echo 'Logs: .runtime/frontend.log and .runtime/backend.log (Redis must be running).'
    ;;
  stop)
    for service in frontend backend; do
      if [[ -f ".runtime/$service.pid" ]]; then
        pid="$(cat ".runtime/$service.pid")"
        # Only terminate our own expected service, not an unrelated reused PID.
        command_line="$(ps -p "$pid" -o args= 2>/dev/null || true)"
        if [[ "$command_line" == *biq-backend* || "$command_line" == *next* ]]; then
          kill "$pid" 2>/dev/null || true
          for attempt in {1..20}; do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.1
          done
        fi
        rm -f ".runtime/$service.pid"
      fi
    done
    ;;
  status)
    for service in backend frontend; do
      if [[ -f ".runtime/$service.pid" ]] && kill -0 "$(cat ".runtime/$service.pid")" 2>/dev/null; then echo "$service running"; else echo "$service stopped"; fi
    done
    ;;
  *) echo 'Usage: scripts/dev.sh start|stop|status'; exit 1 ;;
esac
