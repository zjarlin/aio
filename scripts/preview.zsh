#!/usr/bin/env zsh

# 调用方显式使用其他 shell 时，重新进入脚本声明的 Zsh 运行时。
if [ -z "${ZSH_VERSION:-}" ]; then
  exec zsh "$0" "$@"
fi

set -euo pipefail

script_dir="${0:A:h}"
repository_root="${script_dir:h}"
agent_env_file="${AIO_PROGRAM_AGENT_ENV_FILE:-$HOME/.codex/aio-program-agent-env.zsh}"

if [[ -r "$agent_env_file" ]]; then
  source "$agent_env_file"
fi

cd "$repository_root"
exec cargo run -p az-aio-app
