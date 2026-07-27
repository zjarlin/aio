//! nature 工作台内置的母语源码模板。

/// 可直接编辑并提交的母语源码模板。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NatureSourceTemplate {
    pub key: &'static str,
    pub label: &'static str,
    pub project_id: &'static str,
    pub source_text: &'static str,
}

const ENVIRONMENT_SOURCE: &str =
    include_str!("../../../crates/generated/nature/blueprint-source.txt");

const USER_SOURCE: &str = r#"领域：用户与权限管理

需求：
1. 用户可以注册、登录和维护自己的资料
2. 管理员可以查询、修改和停用用户
3. 用户名和邮箱不能重复，密码不能在列表中展示

建模：用户
1. 用户名：文本，必填，唯一
2. 密码：密码，必填
3. 邮箱：文本，邮箱格式，唯一
4. 权限等级：字典，显示母语标签

操作：
1. 注册用户时校验用户名和邮箱，然后保存用户
2. 登录时校验密码并返回登录结果
3. 管理员可以按用户名和权限等级筛选用户
4. 停用用户前必须确认，停用后刷新用户列表

界面：用户列表
1. 使用表格展示用户名、邮箱、权限等级和状态
2. 顶部提供新增用户操作
3. 支持按用户名和权限等级筛选

界面：用户资料
1. 使用表单管理用户信息
2. 密码只允许通过单独操作修改

导航：
1. 在“组织管理”下面显示“用户管理”
2. 用户列表作为默认页面

权限：
1. 用户只能管理自己的资料
2. 管理员可以管理全部用户
"#;

const DEVICE_SOURCE: &str = r#"领域：边缘设备运行管理

需求：
1. 采集设备在线状态和信号强度，无效数据不能入库
2. 运维人员可以查询离线设备并查看最后数据时间
3. 有效数据到达后更新设备的数据活性时间

建模：设备状态
1. 在线状态：布尔，必填
2. 信号强度：整数，必填，范围 -120 到 0
3. 数据活性时间：时间，必填

操作：
1. 接收状态时校验在线状态和信号强度，然后保存
2. 查询设备时支持按在线状态筛选并返回分页结果
3. 标记设备离线前必须确认，处理后刷新设备列表

界面：设备状态列表
1. 使用表格展示在线状态、信号强度和数据活性时间
2. 支持在线状态筛选、刷新和标记离线操作

界面：设备状态详情
1. 使用只读表单展示设备状态和数据活性时间

导航：
1. 在“设备运维”下面显示“设备状态”
2. 设备状态列表作为默认页面

权限：
1. 运维人员可以查看设备状态
2. 管理员可以标记设备离线

数据获取：
1. 模拟采集提供设备状态原值
2. 在线状态等于在线状态原值
3. 信号强度等于信号强度原值
4. 数据活性时间等于数据活性时间原值
"#;

const SOURCE_TEMPLATES: &[NatureSourceTemplate] = &[
    NatureSourceTemplate {
        key: "environment",
        label: "环境遥测",
        project_id: "环境采集",
        source_text: ENVIRONMENT_SOURCE,
    },
    NatureSourceTemplate {
        key: "user",
        label: "用户资料",
        project_id: "用户管理",
        source_text: USER_SOURCE,
    },
    NatureSourceTemplate {
        key: "device",
        label: "设备状态",
        project_id: "设备管理",
        source_text: DEVICE_SOURCE,
    },
];

pub(crate) fn source_templates() -> &'static [NatureSourceTemplate] {
    SOURCE_TEMPLATES
}

pub(crate) fn select_source_template(key: Option<&str>) -> &'static NatureSourceTemplate {
    key.and_then(|key| SOURCE_TEMPLATES.iter().find(|template| template.key == key))
        .unwrap_or(&SOURCE_TEMPLATES[0])
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use nature_compiler::{CompileRequest, Compiler, CompilerCatalog, MotherTongueInferenceEngine};

    use super::{select_source_template, source_templates};

    #[test]
    fn unknown_template_uses_environment_template() {
        assert_eq!(select_source_template(Some("missing")).key, "environment");
    }

    #[tokio::test]
    async fn every_built_in_template_generates_artifacts() -> anyhow::Result<()> {
        let compiler = Compiler::new(
            Arc::new(MotherTongueInferenceEngine),
            CompilerCatalog::with_fixture_map(),
        );
        for template in source_templates() {
            let result = compiler
                .compile(CompileRequest {
                    source_text: template.source_text.to_string(),
                    previous_blueprint: None,
                })
                .await?;
            assert!(
                result.artifacts.is_some(),
                "内置模板必须能够生成代码: {}: {:?}",
                template.label,
                result.diagnostics
            );
        }
        Ok(())
    }
}
