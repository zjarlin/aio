# 受限运行时适配

Kotlin/Wasm 当前会导入 WASI Preview 1 的 `random_get`。该最小适配器只为 Kotlin 运行时内部哈希种子提供实例内伪随机字节，使最终 Component 保持零宿主导入。

它不提供密码学安全随机数，也不授予插件网络、文件系统、时钟或其他 WASI 能力。插件业务代码不得依赖 `Random.Default` 生成安全令牌。

绑定暂时使用生成器的 `free-all` 内存释放策略。Kotlin 2.4.10 下的实验性 `free-individually` 虽能编译和封装，但在真实 `handle` 调用时会抛出 Wasm 异常；升级前必须重新通过宿主生命周期测试。
