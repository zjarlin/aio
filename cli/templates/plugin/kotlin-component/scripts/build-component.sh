#!/bin/sh

set -eu

KOTLIN_CLI_NO_WELCOME_BANNER=1 ./kotlin build -m component -p wasmWasi -v release

CORE=build/artifacts/CompiledWebArtifact/componentwasmWasirelease/kotlin-output/component.wasm
EMBEDDED=build/component-embedded.wasm
ADAPTER=build/sandbox-preview1-adapter.wasm
OUTPUT=dist/plugin.wasm

mkdir -p build dist
wasm-tools parse runtime/sandbox-preview1-adapter.wat -o "$ADAPTER"
wasm-tools validate --features all "$ADAPTER"
wasm-tools component embed wit/page.wit --world page "$CORE" -o "$EMBEDDED"
wasm-tools component new "$EMBEDDED" \
  --adapt "wasi_snapshot_preview1=$ADAPTER" \
  -o "$OUTPUT"
wasm-tools validate --features component-model "$OUTPUT"

if wasm-tools component wit "$OUTPUT" | grep -q '^[[:space:]]*import '; then
  echo "最终 Component 不得包含宿主导入" >&2
  exit 1
fi

echo "$OUTPUT"
