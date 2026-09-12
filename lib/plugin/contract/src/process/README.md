# Process 宿主协议

Configuration 由宿主写入私有只读挂载，包含当前租户的数据角色、派生密钥、入口票据和 Unix broker 地址。模型出站与跨插件调用由宿主执行并重新鉴权。ServiceRequest 的交互身份只在对应入站请求有效期内有效，后台调用使用独立服务身份。
