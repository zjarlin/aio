# 在线发布

插件仓库的 Git push 可以触发预构建 artifact 在线更新，但 Git 不再是公网请求期的依赖。市场条目、已验证清单、锁定提交、artifact 摘要、页面定义和生命周期事件都保存 PostgreSQL；Git 只保留代码审计和可复现构建来源。

## 发布凭证

有 `plugin:manage` 的当前租户管理员为一个 HTTPS Git 来源创建发布凭证：

```bash
curl --fail --cookie "aio_session=<登录会话>" \
  --header 'content-type: application/json' \
  --data '{"git":"https://github.com/example/aio-plugin-orders.git"}' \
  https://aio.addzero.site/api/runtime/publish-credentials
```

响应的 `token` 只返回一次。把它保存为该插件仓库的 `AIO_PLUGIN_PUBLISH_TOKEN` Actions secret，并把 `https://aio.addzero.site/api/runtime/plugins/publish` 保存为 `AIO_PLUGIN_PUBLISH_URL` Actions variable。凭证被固定到创建时的租户和 Git 地址，不能更新其他来源或租户；轮换凭证会立即使旧 token 失效，撤销接口是：

```bash
curl --fail --request DELETE --cookie "aio_session=<登录会话>" \
  https://aio.addzero.site/api/runtime/publish-credentials/<credential-id>
```

## CI 请求

发布仅接受完整 `GITHUB_SHA`、`aio-plugin.toml`、Base64 artifact 和 SHA-256。artifact 必须是该 SHA 中已跟踪的原始字节；CI 可以重建源码验证工具链，但必须在发布前恢复提交内 artifact。宿主不会运行 `pnpm`、Gradle、Cargo 或仓库脚本。请求先把清单元数据和 artifact 写入 PostgreSQL/版本缓存并返回 `job_id`，随后由后台队列校验 PageDefinition、Component ABI 或 process 清单，并执行对应的实例健康检查；校验或激活失败时保持旧活动 revision。

统一使用 `aio plugin publish` 组装请求，CI 不应自行拼接 JSON、Base64、摘要和轮询脚本。该命令先执行共享协议校验，然后逐字节比较工作树中的清单和 artifact 与当前 Git 提交中的 blob；任何未提交、构建后未恢复或 SHA 不一致都会在上传前失败。

```yaml
- name: 检出固定版本的 AIO CLI
  uses: actions/checkout@v7
  with:
    repository: zjarlin/aio
    ref: ${{ vars.AIO_CLI_REVISION }} # 必须配置为审核过的完整提交 SHA
    path: aio-host-source
    submodules: recursive

- name: 验证 AIO 插件协议
  run: cargo +nightly run --manifest-path aio-host-source/Cargo.toml --bin aio -- plugin validate "$GITHUB_WORKSPACE"

- name: 发布已验证的插件
  if: github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.AIO_PLUGIN_PUBLISH_URL != ''
  env:
    AIO_PLUGIN_PUBLISH_URL: ${{ vars.AIO_PLUGIN_PUBLISH_URL }}
    AIO_PLUGIN_PUBLISH_TOKEN: ${{ secrets.AIO_PLUGIN_PUBLISH_TOKEN }}
  run: |
    if [ -z "$AIO_PLUGIN_PUBLISH_TOKEN" ]; then
      exit 0
    fi
    cargo +nightly run --manifest-path aio-host-source/Cargo.toml --bin aio -- plugin publish "$GITHUB_WORKSPACE"
```

发布接口会立即返回当前状态（`queued`、`running`、`active` 或 `failed`）：

```json
{"data":{"job_id":"...","state":"queued","detail":"已持久化 artifact，等待后台验证","revision":"<GITHUB_SHA>"}}
```

使用来源绑定的发布 token 或同一租户的管理员登录会话查询后台进度；`failed` 响应包含原因，上一活动版本继续提供服务：

```bash
curl --fail --cookie "aio_session=<登录会话>" \
  https://aio.addzero.site/api/runtime/publish-jobs/<job_id>

curl --fail --header "authorization: Bearer <发布令牌>" \
  https://aio.addzero.site/api/runtime/publish-jobs/<job_id>
```

在线发布要求在 `aio-plugin.toml` 的同一份可校验清单中声明市场元数据。宿主从已校验的清单写入 PostgreSQL，因此 Git 提交、Wasm、能力声明、页面和市场卡片会锁定在同一个 revision：

```toml
[plugin.marketplace]
title = "订单中心"
summary = "按租户隔离的订单处理页面与服务。"
license = "MIT"
tags = ["orders", "wasm-component"]
```

市场页面只读 PostgreSQL 缓存：远程 HTTPS/Git registry 会后台刷新，超时只保留上一次成功索引，不影响市场页面和活动插件。
