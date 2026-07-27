#!/usr/bin/env zsh

set -euo pipefail

script_dir="${0:A:h}"
repository_root="${script_dir:h}"
nature_env_file="${NATURE_COMPILER_ENV_FILE:-$HOME/.codex/nature-compiler-env.zsh}"

if [[ -r "$nature_env_file" ]]; then
  source "$nature_env_file"
fi

cd "$repository_root"
exec cargo run -p az-aio-web
