# 构建工作进程

`aio-delivery` 从宿主领取持久化任务，在受限 Docker 容器中构建锁定的源码提交，随后在容器外打包并提交结果。发布令牌和宿主接口不传入构建容器。

必需环境变量：`AIO_DELIVERY_TOKEN`、三种 `AIO_BUILD_IMAGE_RUST/KOTLIN/TYPESCRIPT`（必须是固定 digest）。可选 `AIO_DELIVERY_URL`（默认 `http://127.0.0.1:3080`）、`AIO_DELIVERY_ROOT`（默认 `/opt/aio-delivery`）、`AIO_CLI`（默认 `aio`）。
