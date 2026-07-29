# Asset Hub

Cargo 工件：`az-asset-hub`

本插件解决资产登记、查询和本地技能资产扫描问题，拥有 `biz_asset_hub_*` PostgreSQL 模型以及 `/api/asset-hub/*` 后端接口。

它不拥有页面 renderer、不负责通用上传或其他插件的数据。资产页面由数据库 ProgramGraph 绑定本插件 API。

```bash
cargo test -p az-asset-hub
```
