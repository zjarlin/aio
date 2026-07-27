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

const USER_SOURCE: &str = r#"需求：用户资料管理
1. 录入用户基础信息
2. 无效数据不能入库

建模：用户
1. 用户名：文本
2. 密码摘要：文本
3. 邮箱：文本
4. 权限等级：文本

数据获取：
1. 模拟采集提供用户资料原值
2. 用户名等于用户名原值
3. 密码摘要等于密码摘要原值
4. 邮箱等于邮箱原值
5. 权限等级等于权限等级原值
"#;

const DEVICE_SOURCE: &str = r#"需求：设备状态采集
1. 采集设备在线状态和信号强度
2. 无效数据不能入库
3. 有效数据到达后更新数据活性时间

建模：设备状态
1. 在线状态：布尔
2. 信号强度：整数，范围 -120 到 0
3. 数据活性时间：时间

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

    use nature_compiler::{
        CompileRequest, Compiler, CompilerCatalog, MotherTongueInferenceEngine,
    };

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
