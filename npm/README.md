# npm 分发

`aio/` 是 `@addzero/aio` 的 JavaScript 入口包；平台二进制包只在发布工作流中由同一份配置生成，不提交构建产物。

GitHub 仓库必须配置名为 `npm` 的受保护 Environment，并为它启用 required reviewers 和发布 tag 限制。首次发布前，在该 Environment 中配置可创建 `@addzero` 公共包的 `NPM_TOKEN`；所有包创建后，再为 `.github/workflows/npm-release.yml` 和 `npm` Environment 配置 npm Trusted Publisher。

发布 tag 必须是 `v<workspace version>`，且所指提交必须属于 `origin/main`。工作流串行发布五个平台包，再发布入口包；失败后使用同一 tag 重跑会跳过已经存在的版本。
