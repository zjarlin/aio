# 工作台与独立持久化页面

工作台的 Counter 使用 Compose 本地状态，点击不调用宿主或后端；切换 Tasks/Counter 保留数值，重新加载插件从零开始。Tasks 的数据操作通过宿主桥访问同包 Ktor 后端。

`counter.html` 单独挂载持久化计数页面，用于 Component 数据库验收，不是工作台本地交互的实现。
