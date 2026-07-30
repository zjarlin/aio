#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

// 模块由编译期扫描统一导出并参与 Rudi 注册。
automod::dir!(pub "src");

rudi::enable! {}
