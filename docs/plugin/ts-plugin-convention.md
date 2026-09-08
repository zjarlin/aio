# TypeScript 插件开发规约

TypeScript 本身不等于 WebAssembly。不要只为获得 `.wasm` 后缀引入不成熟的编译链。

## 客户端

- AIO 原生页面使用语言无关的 `PageDefinition` 与事件协议；TypeScript 负责生成定义和处理事件，不直接修改宿主 DOM。
- 需要完整 Web 框架和自有 DOM 时，产出隔离页面，由宿主以受限页面容器装载；必须声明网络、剪贴板、下载等权限。
- 只有所用编译器能够生成并通过 `aio:plugin/page@1` Component ABI 校验时，才声明 `wasm-component`。

## 服务端

依赖 Node.js、数据库驱动、队列或长期任务时声明 `process`，提交锁文件并提供 `/health`。宿主注入端口和临时数据目录，插件不得占用固定宿主端口，也不得把密钥写进仓库。

## 验证

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm lint
pnpm test
pnpm build
```

安装阶段不执行任意生命周期脚本；确需原生构建时必须在市场条目中标记，交由隔离构建器显式运行。产物、来源提交和依赖锁三者必须可追溯。
