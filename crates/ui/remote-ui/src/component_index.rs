use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result, bail, ensure};
use rudi::Context as RudiContext;

use crate::{
    ComponentCatalogEntry, ComponentDefinition, ComponentEventSpec, ComponentPropertySpec,
    ComponentShape, ComponentSpec, component::DynRemoteComponent,
};

/// 从 Rudi provider 构建出的不可变组件定义。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedComponent {
    canonical_id: String,
    dsl_name: String,
    definition: ComponentDefinition,
    properties: BTreeMap<String, ComponentPropertySpec>,
    events: BTreeMap<String, ComponentEventSpec>,
    semantic_names: Vec<String>,
}

impl IndexedComponent {
    #[must_use]
    pub fn canonical_id(&self) -> &str {
        &self.canonical_id
    }

    #[must_use]
    pub fn dsl_name(&self) -> &str {
        &self.dsl_name
    }

    #[must_use]
    pub const fn shape(&self) -> ComponentShape {
        self.definition.shape
    }

    #[must_use]
    pub fn spec(&self) -> &ComponentSpec {
        &self.definition.spec
    }

    #[must_use]
    pub fn definition(&self) -> &ComponentDefinition {
        &self.definition
    }

    #[must_use]
    pub fn properties(&self) -> &BTreeMap<String, ComponentPropertySpec> {
        &self.properties
    }

    #[must_use]
    pub fn semantic_names(&self) -> &[String] {
        &self.semantic_names
    }
}

/// 由当前 Rudi Context 中全部组件 provider 派生出的只读查询索引。
#[derive(Clone, Debug, Default)]
pub struct ComponentIndex {
    by_dsl_name: BTreeMap<String, Arc<IndexedComponent>>,
    by_canonical_id: BTreeMap<String, Arc<IndexedComponent>>,
}

impl ComponentIndex {
    /// 从现有 Rudi Context 收集全部远程组件。
    pub fn from_context(context: &mut RudiContext) -> Result<Self> {
        let provider_names = context
            .get_providers_by_type::<DynRemoteComponent>()
            .into_iter()
            .map(|provider| provider.definition().key.name.to_string())
            .collect::<Vec<_>>();
        ensure!(!provider_names.is_empty(), "Rudi 中没有注册远程 UI 组件");

        let mut index = Self::default();
        for provider_name in provider_names {
            let component = context
                .resolve_option_with_name::<DynRemoteComponent>(provider_name.clone())
                .with_context(|| format!("无法解析远程 UI 组件 provider: {provider_name}"))?;
            let definition = component.definition();
            let properties = component.properties();
            let events = component.events();
            let semantic_names = component.semantic_names();
            index.insert(
                provider_name,
                definition,
                properties,
                events,
                semantic_names,
            )?;
        }
        Ok(index)
    }

    /// 按 DSL 中的短名称解析组件。
    pub fn resolve(&self, dsl_name: &str) -> Result<&IndexedComponent> {
        self.by_dsl_name
            .get(dsl_name)
            .map(Arc::as_ref)
            .with_context(|| format!("未注册的远程 UI 组件: {dsl_name}"))
    }

    /// 返回浏览器 renderer 使用的 canonical ID 到样式定义映射。
    #[must_use]
    pub fn browser_catalog(&self) -> BTreeMap<String, ComponentCatalogEntry> {
        self.by_canonical_id
            .iter()
            .map(|(id, component)| {
                (
                    id.clone(),
                    ComponentCatalogEntry {
                        shape: component.shape(),
                        spec: component.spec().clone(),
                        properties: component.properties.clone(),
                        events: component.events.clone(),
                    },
                )
            })
            .collect()
    }

    /// 返回自然编译器可消费的组件语义目录。
    pub fn semantic_catalog(&self) -> Vec<(String, String, Vec<String>)> {
        self.by_canonical_id
            .values()
            .map(|component| {
                (
                    component.canonical_id.clone(),
                    component.dsl_name.clone(),
                    component.semantic_names.clone(),
                )
            })
            .collect()
    }

    /// 按正式配置中的 canonical ID 解析组件。
    pub fn resolve_canonical(&self, canonical_id: &str) -> Result<&IndexedComponent> {
        self.by_canonical_id
            .get(canonical_id)
            .map(Arc::as_ref)
            .with_context(|| format!("未注册的远程 UI 组件路径: {canonical_id}"))
    }

    fn insert(
        &mut self,
        canonical_id: String,
        definition: ComponentDefinition,
        properties: BTreeMap<String, ComponentPropertySpec>,
        events: BTreeMap<String, ComponentEventSpec>,
        semantic_names: Vec<String>,
    ) -> Result<()> {
        let dsl_name = dsl_name_from_canonical_id(&canonical_id)?;
        if let Some(existing) = self.by_dsl_name.get(&dsl_name) {
            bail!(
                "远程 UI 组件短名称冲突: {dsl_name}; {} 与 {canonical_id}",
                existing.canonical_id()
            );
        }

        let component = Arc::new(IndexedComponent {
            canonical_id: canonical_id.clone(),
            dsl_name: dsl_name.clone(),
            definition,
            properties,
            events,
            semantic_names,
        });
        self.by_dsl_name.insert(dsl_name, Arc::clone(&component));
        self.by_canonical_id.insert(canonical_id, component);
        Ok(())
    }
}

fn dsl_name_from_canonical_id(canonical_id: &str) -> Result<String> {
    let module_name = canonical_id
        .rsplit("::")
        .next()
        .filter(|value| !value.is_empty())
        .with_context(|| format!("组件 provider 路径无效: {canonical_id}"))?;
    Ok(module_name.replace('_', "-"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rudi_provider_path_derives_dsl_name_and_catalog_key() -> Result<()> {
        let mut context = RudiContext::auto_register();
        let components = ComponentIndex::from_context(&mut context)?;
        let card = components.resolve("card")?;

        // DSL 使用短名称，协议和 catalog 使用同一个编译时 provider 路径。
        assert!(card.canonical_id().ends_with("::components::card"));
        assert!(
            components
                .browser_catalog()
                .contains_key(card.canonical_id())
        );
        Ok(())
    }
}
