# 构建工作进程

`aio-delivery` 从宿主领取持久化任务，在受限 Docker 容器中构建锁定的源码提交，随后在容器外打包并提交结果。发布令牌和宿主接口不传入构建容器。

必需环境变量：`AIO_DELIVERY_TOKEN`、三种 `AIO_BUILD_IMAGE_RUST/KOTLIN/TYPESCRIPT`（必须是固定 digest）。可选 `AIO_DELIVERY_URL`（默认 `http://127.0.0.1:3080`）、`AIO_DELIVERY_ROOT`（默认 `/opt/aio-delivery`）、`AIO_CLI`（默认 `aio`）。

## 部署

构建 DNS 可通过 `AIO_BUILD_DNS` 指定；部署网络中需要域名地址覆盖时，可配置 `AIO_BUILD_HOSTS=github.com=IP`，多个地址以逗号分隔。此设置仅属于构建服务环境，保留正常 HTTPS 证书校验，不写入插件源码；重新配置服务时保留。地址应由运维按实际连通性维护。下载设置低速超时，网络失败按任务持久化退避并保留产物，其他源码仍可领取构建。

Rust 适配器使用固定 nightly 的 Cargo `shallow-deps` 功能，只拉取锁定依赖提交，避免下载共享仓库的全部历史；构建结果仍绑定完整源码 SHA。

先部署共享协议与宿主，再安装 `aio-delivery`、`aio` 到 `/opt/aio-delivery/bin`。`install-git.sh` 为旧系统编译独立 Git，依赖 gcc、make、libcurl/openssl/zlib/expat 开发包，不覆盖系统 Git。镜像构建使用 `docker build --network host --build-arg BASE_IMAGE=<固定摘要>`；源码任务仍在独立网络的受限容器内执行。

在服务器执行 `node configure.cjs <Rust image ID> <Kotlin image ID> <TypeScript image ID>`，为宿主和工作进程生成一次性共享凭据。GitHub 发现凭据通过标准输入交给 `configure-github.cjs`，仅保存到宿主环境，不传给源码容器。安装 `aio-delivery.service` 后重启宿主并启用构建服务。

`inspect.cjs` 只读取仓库、任务和租户升级状态；传入任务 ID 可查看该任务的完整错误。运维依赖可用 `npm --prefix /opt/aio-delivery/ops install pg@8.16.3 --save-exact --ignore-scripts` 安装。脚本读取服务器已有数据库配置，不输出凭据。
