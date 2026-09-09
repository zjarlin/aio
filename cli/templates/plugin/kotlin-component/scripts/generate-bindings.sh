#!/bin/sh

set -eu

WIT_BINDGEN=${WIT_BINDGEN:-wit-bindgen}
OUTPUT=component/src/__PACKAGE_PATH__/bindings

exec "$WIT_BINDGEN" kotlin \
  --kotlin-package-name __PACKAGE_NAME__.bindings \
  --kotlin-imports __PACKAGE_NAME__.component.PageRootFunctionsExportsImpl \
  --declaration-visibility internal \
  --cabi-realloc-freeing-strategy free-all \
  --out-dir "$OUTPUT" \
  "$@" \
  wit/page.wit
