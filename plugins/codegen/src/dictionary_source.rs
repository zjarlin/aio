//! AIO PostgreSQL 字典到可移植源码 bundle 的转换。

use anyhow::{Context, bail};
use az_aio_platform::system::store::SystemAdminStore;
use az_dict_spec::specification::{DictionaryItemSpec, DictionarySpec, RawValueKind};
use az_micro_dict::contribution::{
    DictBuildGenerator, DictSourceBundle, DictionaryContribution, StaticDictionaryContributor,
};
use convert_case::{Case, Casing};

/// 从 `sys_dict_*` 读取全部启用字典并生成内存源码。
pub async fn enabled_dictionary_bundle(
    store: &SystemAdminStore,
) -> anyhow::Result<Option<DictSourceBundle>> {
    let dictionary_types = store.list_dictionary_types().await?;
    let mut contributions = Vec::new();
    for dictionary_type in dictionary_types
        .into_iter()
        .filter(|dictionary_type| dictionary_type.status == "enabled")
    {
        let page = store
            .list_dictionary_items(&dictionary_type.id, None, 0, 100)
            .await?;
        if page.t > page.d.len() {
            bail!(
                "字典 {} 超过单次生成上限 100，请拆分字典后再生成",
                dictionary_type.name
            );
        }
        let raw_value_kind = match dictionary_type.raw_value_kind.as_str() {
            "int" => RawValueKind::Int,
            "string" => RawValueKind::String,
            other => bail!(
                "字典 {} 使用不支持的原始值类型: {other}",
                dictionary_type.name
            ),
        };
        let items =
            page.d
                .into_iter()
                .filter(|item| item.status == "enabled")
                .map(|item| {
                    let (raw_int_value, raw_text_value) = match raw_value_kind {
                        RawValueKind::Int => (
                            Some(item.raw_value.parse::<i64>().with_context(|| {
                                format!("字典项 {} 的整数原始值无效", item.label)
                            })?),
                            None,
                        ),
                        RawValueKind::String => (None, Some(item.raw_value)),
                    };
                    let meta = if item.meta_json.trim().is_empty() {
                        None
                    } else {
                        Some(
                            serde_json::from_str(&item.meta_json)
                                .with_context(|| format!("字典项 {} 的元数据无效", item.label))?,
                        )
                    };
                    Ok(DictionaryItemSpec {
                        code: item.code,
                        label: item.label,
                        description: Some(item.description),
                        raw_int_value,
                        raw_text_value,
                        sort_index: item.sort_index,
                        enabled: true,
                        meta,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
        contributions.push(DictionaryContribution::new(
            dictionary_type.code.to_case(Case::Pascal),
            DictionarySpec {
                code: dictionary_type.code,
                name: dictionary_type.name,
                description: Some(dictionary_type.description),
                scope: dictionary_type.scope,
                raw_value_kind,
                open_enum: dictionary_type.open_enum,
                unknown_variant: dictionary_type.open_enum.then(|| "Other".to_string()),
                sort_index: dictionary_type.sort_index,
                items,
            },
        ));
    }
    if contributions.is_empty() {
        return Ok(None);
    }
    DictBuildGenerator::new()
        .add_contributor(StaticDictionaryContributor::new(contributions))
        .generate_bundle()
        .context("生成 AIO 字典源码 bundle 失败")
        .map(Some)
}
