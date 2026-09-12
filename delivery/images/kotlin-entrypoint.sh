#!/bin/sh
set -eu

# 源码抓取阶段没有任务缓存；仅在构建阶段预置经过镜像摘要锁定的工具。
if [ -d /cache/target ]; then
    download_cache=/cache/.cache/JetBrains/Kotlin/download.cache
    mkdir -p "$download_cache"
    for archive in /opt/aio-kotlin-downloads/*; do
        destination="$download_cache/${archive##*/}"
        if [ ! -f "$destination" ]; then
            cp "$archive" "$destination.new"
            mv "$destination.new" "$destination"
        fi
    done
fi
exec "$@"
