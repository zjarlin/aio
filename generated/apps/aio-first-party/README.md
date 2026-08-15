# AIO跨平台

该目录由 AIO ApplicationCompiler 从已发布的 ProgramDefinition 生成。

应用标识：`aio-first-party`  
Cargo 包：`az-app-aio-first-party`

## Web

```bash
dx serve --package az-app-aio-first-party --platform web --features web
```

## Desktop

```bash
AIO_API_BASE_URL=http://127.0.0.1:8080 cargo run -p az-app-aio-first-party --no-default-features --features desktop
```

## Server

```bash
cargo run -p az-app-aio-first-party --no-default-features --features server
```

## Container

从仓库根目录执行：

```bash
docker build -f generated/apps/aio-first-party/Dockerfile -t az-app-aio-first-party .
```
