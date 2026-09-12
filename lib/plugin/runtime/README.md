# Wasm 宿主

提供按调用上下文隔离的 Component 执行与能力代理，不依赖业务插件。数据库连接必须使用宿主为当前插件和租户配置的专属最小权限角色，不能传入平台管理连接。

每次调用临时持有完整执行状态，只有成功结束才归还实例；取消、超时或陷阱会销毁状态，未提交事务进入回滚。失效实例不能复用。空值参数使用 PostgreSQL 的上下文类型推导，不强制转换为文本。

`cargo test -p az-plugin-runtime` 执行无外部依赖的测试。完整测试需要一次性 PostgreSQL（已撤销 public schema 的 PUBLIC 权限）以及已构建的 Kotlin 示例，三个路径/地址通过环境变量指定：

```sh
export AIO_TEST_DATABASE_URL=postgres://test_admin@127.0.0.1:55432/postgres
export AIO_TEST_COMPONENT=/absolute/path/aio-plugin-kmp-example/dist/plugin.wasm
export AIO_TEST_MIGRATION=/absolute/path/aio-plugin-kmp-example/backend/migrations/0001_counter.sql
cargo test -p az-plugin-runtime -- --include-ignored
```

这些测试创建随机来源的独立角色和 schema，仅对一次性测试库运行。初始 schema provisioner 不等于生产迁移管理器；持久绑定、迁移版本、注册表与内存切换的一致性、公网切换仍由 `docs/refactor/README.md` 跟踪。

## 整包执行槽

`ComponentSlot` 是按来源 UUID 和租户绑定的进程内执行单元。它只接受 `az-plugin-bundle::VerifiedBundle`：校验能力授予、宿主版本、实例描述和前端入口，运行 prepare/health/activate/health，再排空在途请求并一起交换页面资源、描述和实例。失败或准备被取消时保留旧版本。调用必须匹配当前整包摘要及租户；升级后旧摘要失效，停用后拒绝调用。旧实例清理失败只记录日志并销毁，不撤销已激活的新版本。

新旧迁移文件集合必须完全一致，否则拒绝在线替换，交由维护迁移流程。该单元不提供持久化注册表、登录授权或迁移执行器，不应直接暴露成公网安装接口。首次实例化前数据库必须由可信宿主完成受控初始化；停用不删除业务数据。`ReleaseSnapshot` 是只读资源快照，不是认证票据。

Rust 和 Kotlin SDK 的 WASI 仅开放受限 I/O、时间、安全随机及空环境。标准输出和错误输出各有 4 KiB 缓冲，不继承宿主环境、文件预打开目录、终端或网络。
