# TypeScript 插件开发规约

TypeScript 本身不等于 WebAssembly。不要只为获得 `.wasm` 后缀引入不成熟的编译链。

可安装的参考仓库是 [aio-plugin-ts-component](https://github.com/zjarlin/aio-plugin-ts-component)。当前参考工具链固定为 pnpm 10.33.2、TypeScript 7.0.2、JCO 1.32.1 和 ComponentizeJS 0.22.0；更新任何版本都应作为一次明确的插件发布。

## 客户端

- AIO 原生页面使用语言无关的 `PageDefinition` 与 [`aio:plugin/page@1`](wit/page.wit)；TypeScript 产物通过 `definition` 生成定义并通过 `handle` 处理受限请求，不直接修改宿主 DOM。
- 需要完整 Web 框架和自有 DOM 时，产出隔离页面，由宿主以受限页面容器装载；必须声明网络、剪贴板、下载等权限。
- 只有所用编译器能够生成并通过 `aio:plugin/page@1` Component ABI 校验时，才声明 `wasm-component`。
- 纯页面和轻量请求插件应关闭 `stdio`、`random`、`clocks`、`http` 和 `fetch-event`，使产物不导入 WASI 能力。

## 服务端

依赖 Node.js、数据库驱动、队列或长期任务时应声明 `process`，提交锁文件并提供 `/health`。公网宿主在进程隔离监督器完成前会拒绝该目标；插件不得占用固定宿主端口、自行守护或把密钥写进仓库。

## 验证

```bash
corepack enable
pnpm install --frozen-lockfile --ignore-scripts
pnpm typecheck
pnpm test
pnpm build
wasm-tools validate --features component-model dist/plugin.wasm
wasm-tools component wit dist/plugin.wasm
```

安装阶段不执行任意生命周期脚本；确需原生构建时必须在市场条目中标记，交由隔离构建器显式运行。产物、来源提交和依赖锁三者必须可追溯。

ComponentizeJS 目前仍是实验性工具；SpiderMonkey 的预初始化快照不保证字节级可重复。因此不得仅提交源码并让生产安装器现场构建：必须同时提交 Component artifact 和 pnpm 锁文件，并由完整 Git 提交 SHA 锁定实际运行字节。
