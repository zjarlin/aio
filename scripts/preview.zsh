#!/usr/bin/env zsh

set -euo pipefail

script_dir="${0:A:h}"
repository_root="${script_dir:h}"
agent_env_file="${AIO_PROGRAM_AGENT_ENV_FILE:-$HOME/.codex/aio-program-agent-env.zsh}"

if [[ -r "$agent_env_file" ]]; then
  source "$agent_env_file"
fi

cd "$repository_root"
exec cargo run -p az-aio-app
