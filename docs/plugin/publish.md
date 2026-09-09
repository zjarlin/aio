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

发布仅接受完整 `GITHUB_SHA`、`aio-plugin.toml`、Base64 artifact 和 SHA-256。宿主不会运行 `pnpm`、Gradle、Cargo 或仓库脚本。请求先把清单元数据和 artifact 写入 PostgreSQL/版本缓存并返回 `job_id`，随后由后台队列重新校验 WIT ABI、无导入限制、页面定义和租户实例健康检查；校验或激活失败时保持旧活动 revision。

```yaml
- name: 发布已验证的 Component
  if: github.event_name == 'push' && github.ref == 'refs/heads/main' && vars.AIO_PLUGIN_PUBLISH_URL != '' && secrets.AIO_PLUGIN_PUBLISH_TOKEN != ''
  env:
    AIO_PLUGIN_PUBLISH_URL: ${{ vars.AIO_PLUGIN_PUBLISH_URL }}
    AIO_PLUGIN_PUBLISH_TOKEN: ${{ secrets.AIO_PLUGIN_PUBLISH_TOKEN }}
  run: |
    artifact_file="$RUNNER_TEMP/plugin.wasm.base64"
    base64 --wrap=0 dist/plugin.wasm | tr -d '\n' > "$artifact_file"
    artifact_sha256="$(sha256sum dist/plugin.wasm | awk '{print $1}')"
    jq -n \
      --arg git "${{ github.server_url }}/${{ github.repository }}.git" \
      --arg rev "$GITHUB_SHA" \
      --rawfile manifest aio-plugin.toml \
      --rawfile artifact_base64 "$artifact_file" \
      --arg artifact_sha256 "$artifact_sha256" \
      '{git:$git, rev:$rev, manifest_toml:$manifest, artifact_base64:$artifact_base64, artifact_sha256:$artifact_sha256}' \
      | curl --fail-with-body --show-error --request POST \
          --header "authorization: Bearer $AIO_PLUGIN_PUBLISH_TOKEN" \
          --header 'content-type: application/json' \
          --data-binary @- "$AIO_PLUGIN_PUBLISH_URL"
```

发布接口会立即返回当前状态（`queued`、`running`、`active` 或 `failed`）：

```json
{"job_id":"...","state":"queued","revision":"<GITHUB_SHA>"}
```

使用同一租户的登录会话查询后台进度；`failed` 响应包含原因，上一活动版本继续提供服务：

```bash
curl --fail --cookie "aio_session=<登录会话>" \
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
