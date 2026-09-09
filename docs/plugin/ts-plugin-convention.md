# TypeScript 插件开发规约

TypeScript 本身不等于 WebAssembly。不要只为获得 `.wasm` 后缀引入不成熟的编译链。

初始化当前稳定的 TypeScript Component 仓库：

```bash
aio plugin init ../aio-plugin-ts --title "TS 页面" --language typescript --runtime wasm-component
```

可安装的参考仓库是 [aio-plugin-ts-component](https://github.com/zjarlin/aio-plugin-ts-component) 和 [aio-plugin-ts-service](https://github.com/zjarlin/aio-plugin-ts-service)。当前参考工具链固定为 pnpm 10.33.2、TypeScript 7.0.2、JCO 1.32.1 和 ComponentizeJS 0.22.0；更新任何版本都应作为一次明确的插件发布。

## 客户端

- AIO 原生页面使用语言无关的 `PageDefinition` 与 [`aio:plugin/page@1`](wit/page.wit)；TypeScript 产物通过 `definition` 生成定义并通过 `handle` 处理受限请求，不直接修改宿主 DOM。
- 需要宿主持有按钮布局与持久状态时使用 `actions` 页面体。`handle` 接收宿主注入的 `page_action` 请求和当前 `body.state`，并在响应 `body` 中返回 `{"body": <PageBody>}`；插件应实现无状态 reducer，不要信任浏览器提交的租户、用户或页面状态。
- 需要在账户区提供入口时，在子插件清单写 `account_actions = ["页面 ID"]`。该入口只能跳转到同一 Component 已声明的页面，标题、图标和权限均由页面定义推导。
- 需要完整 Web 框架和自有 DOM 时，产出隔离页面，由宿主以受限页面容器装载；必须声明网络、剪贴板、下载等权限。
- 只有所用编译器能够生成并通过 `aio:plugin/page@1` Component ABI 校验时，才声明 `wasm-component`。
- 纯页面和轻量请求插件应关闭 `stdio`、`random`、`clocks`、`http` 和 `fetch-event`，使产物不导入 WASI 能力。

## 服务端

依赖 Node.js、数据库驱动、队列或长期任务时应声明 `process`，提交锁文件并提供健康检查。公网宿主已经启用隔离进程监督器，但当前只授予零额外能力档；插件必须使用预置 digest 镜像，监听 `AIO_PLUGIN_PORT`，实现 `GET /aio/definition` 和清单声明的路由，不得占用固定宿主端口、自行守护或把密钥写进仓库。

## 验证

使用 `aio plugin schema schemas` 获取正式 JSON Schema。TypeScript 类型生成和结构校验应以这些文件为输入，不复制宿主私有模型。

```bash
corepack enable
pnpm install --frozen-lockfile --ignore-scripts
pnpm typecheck
pnpm test
pnpm build
wasm-tools validate --features component-model dist/plugin.wasm
wasm-tools component wit dist/plugin.wasm
aio plugin validate
```

安装阶段不执行任意生命周期脚本；确需原生构建时必须在市场条目中标记，交由隔离构建器显式运行。产物、来源提交和依赖锁三者必须可追溯。

ComponentizeJS 目前仍是实验性工具；SpiderMonkey 的预初始化快照不保证字节级可重复。因此不得仅提交源码并让生产安装器现场构建：必须同时提交 Component artifact 和 pnpm 锁文件，并由完整 Git 提交 SHA 锁定实际运行字节。

Node `process` 插件同样必须提交编译后的 JavaScript artifact 与 pnpm 锁文件；生产安装器只读取 artifact，不执行 `pnpm`、`npm` 或仓库脚本。可执行示例见 [aio-plugin-ts-service](https://github.com/zjarlin/aio-plugin-ts-service)。
