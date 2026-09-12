#!/bin/sh
set -eu
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm test
