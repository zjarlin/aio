#!/usr/bin/env zsh

# 调用方显式使用其他 shell 时，重新进入脚本声明的 Zsh 运行时。
if [ -z "${ZSH_VERSION:-}" ]; then
  exec zsh "$0" "$@"
fi

set -euo pipefail

script_dir="${0:A:h}"
repository_root="${script_dir:h}"
agent_env_file="${AIO_PROGRAM_AGENT_ENV_FILE:-$HOME/.codex/aio-program-agent-env.zsh}"
relay_pid=""

if [[ -r "$agent_env_file" ]]; then
  source "$agent_env_file"
fi


cd "$repository_root"
database_transport="${AIO_DATABASE_TRANSPORT:-auto}"
relay_port="${AIO_DATABASE_RELAY_PORT:-15434}"
socks_host="${AIO_DATABASE_SOCKS_HOST:-127.0.0.1}"
socks_port="${AIO_DATABASE_SOCKS_PORT:-7890}"
node_bin="${AIO_NODE_BIN:-node}"
psql_bin="${AIO_PSQL_BIN:-/opt/homebrew/opt/libpq/bin/psql}"

database_url="$(sed -n -E 's/^AZ_AIO_DATABASE_URL=(.*)$/\1/p' .env | head -n 1)"
if [[ -z "$database_url" ]]; then
  print -u2 "缺少 .env 中的 AZ_AIO_DATABASE_URL"
  exit 1
fi
repository_web_port="$(sed -n -E 's/^AZ_AIO_WEB_PORT=(.*)$/\1/p' .env | head -n 1)"
web_port="${repository_web_port:-${AZ_AIO_WEB_PORT:-8080}}"

database_authority="${${database_url#*://}%%/*}"
database_host_port="${database_authority##*@}"
database_host="${database_host_port%%:*}"
database_port="${database_host_port##*:}"
database_path="${${database_url#*://}#*/}"
database_name="${database_path%%\?*}"
relay_database_url="${database_url/$database_host_port/127.0.0.1:$relay_port}"
expected_database_identity="${database_name}|${database_host}|${database_port}"

if [[ ! -x "$psql_bin" ]]; then
  psql_bin="$(command -v psql || true)"
fi

if [[ -z "$psql_bin" ]]; then
  print -u2 "未找到 PostgreSQL psql 客户端，无法验证 AIO PostgreSQL"
  exit 1
fi

database_identity() {
  PGCONNECT_TIMEOUT=3 "$psql_bin" "$1" -X -Atq \
    -c "select current_database() || '|' || host(inet_server_addr()) || '|' || inet_server_port()" \
    2>/dev/null || true
}

prepare_web_port() {
  local listener_pids
  local listener_pid
  local listener_cwd
  local listener_executable

  listener_pids="$(lsof -nP -tiTCP:"$web_port" -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -z "$listener_pids" ]]; then
    return
  fi

  for listener_pid in ${(f)listener_pids}; do
    listener_cwd="$(lsof -a -p "$listener_pid" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1)"
    listener_executable="$(lsof -a -p "$listener_pid" -d txt -Fn 2>/dev/null | sed -n 's/^n//p' | head -n 1)"
    if [[ "$listener_cwd" != "$repository_root" || "${listener_executable:t}" != "az-aio-app" ]]; then
      print -u2 "端口 $web_port 已被其他进程占用: PID $listener_pid"
      exit 1
    fi
  done

  print "停止旧 AIO 进程: PID ${listener_pids//$'\n'/, }"
  for listener_pid in ${(f)listener_pids}; do
    kill "$listener_pid" 2>/dev/null || true
  done

  for attempt in {1..50}; do
    if ! lsof -nP -iTCP:"$web_port" -sTCP:LISTEN >/dev/null 2>&1; then
      return
    fi
    sleep 0.1
  done

  print -u2 "旧 AIO 进程未在 5 秒内释放端口 $web_port"
  exit 1
}

if [[ "$database_transport" == "auto" || "$database_transport" == "direct" ]]; then
  if [[ "$(database_identity "$database_url")" == "$expected_database_identity" ]]; then
    prepare_web_port
    exec cargo run -p az-aio-app
  fi
  if [[ "$database_transport" == "direct" ]]; then
    print -u2 "直连 AIO PostgreSQL 无法完成数据库握手"
    exit 1
  fi
fi

if [[ "$database_transport" != "auto" && "$database_transport" != "relay" ]]; then
  print -u2 "AIO_DATABASE_TRANSPORT 只能为 auto、direct 或 relay"
  exit 1
fi

if [[ ! -x "$node_bin" ]]; then
  node_bin="$(command -v node || true)"
fi

if [[ -z "$node_bin" ]]; then
  print -u2 "未找到 Node.js，无法启动 AIO PostgreSQL SOCKS relay"
  exit 1
fi

cleanup_relay() {
  if [[ -n "$relay_pid" ]]; then
    kill "$relay_pid" 2>/dev/null || true
    wait "$relay_pid" 2>/dev/null || true
  fi
}

trap cleanup_relay EXIT INT TERM

if ! lsof -nP -iTCP@127.0.0.1:"$relay_port" -sTCP:LISTEN >/dev/null 2>&1; then
  "$node_bin" scripts/postgres-socks-relay.mjs \
    --listen-host 127.0.0.1 \
    --listen-port "$relay_port" \
    --proxy-host "$socks_host" \
    --proxy-port "$socks_port" \
    --target-host "$database_host" \
    --target-port "$database_port" &
  relay_pid="$!"
fi

relay_ready=""
for attempt in {1..10}; do
  if [[ "$(database_identity "$relay_database_url")" == "$expected_database_identity" ]]; then
    relay_ready="yes"
    break
  fi
  if [[ -n "$relay_pid" ]] && ! kill -0 "$relay_pid" 2>/dev/null; then
    wait "$relay_pid" || true
    print -u2 "无法启动 AIO PostgreSQL SOCKS relay"
    exit 1
  fi
  sleep 1
done

if [[ -z "$relay_ready" ]]; then
  print -u2 "AIO PostgreSQL SOCKS relay 无法完成数据库握手"
  exit 1
fi

prepare_web_port
AZ_AIO_DATABASE_URL_OVERRIDE="$relay_database_url" cargo run -p az-aio-app
