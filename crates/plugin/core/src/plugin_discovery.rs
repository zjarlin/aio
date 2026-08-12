//! 插件的 Rudi 发现和统一安装。

use anyhow::{Context as _, Result, ensure};

use crate::{DynPlugin, Plugin, resolve_plugin_order};

/// 从 Rudi 容器收集全部插件并校验类型身份。
pub fn discover_plugins<T: 'static>(context: &mut rudi::Context) -> Result<Vec<DynPlugin<T>>> {
    let registration_names = context
        .get_providers_by_type::<DynPlugin<T>>()
        .into_iter()
        .map(|provider| provider.definition().key.name.clone())
        .collect::<Vec<_>>();
    let mut plugins = Vec::with_capacity(registration_names.len());

    for registration_name in registration_names {
        let Some(plugin) =
            context.resolve_option_with_name::<DynPlugin<T>>(registration_name.clone())
        else {
            continue;
        };
        ensure!(
            registration_name.as_ref() == plugin.key(),
            "Rudi 注册名与插件类型标识不一致: 注册名={}, 类型标识={}",
            registration_name,
            plugin.key()
        );
        plugins.push(plugin);
    }

    Ok(plugins)
}

/// 过滤已启用插件，解析依赖顺序后逐个安装。
pub async fn install_plugins<T: 'static>(
    target: &mut T,
    plugins: Vec<DynPlugin<T>>,
) -> Result<Vec<&'static str>> {
    let enabled_plugins = plugins
        .into_iter()
        .filter(|plugin| plugin.enabled(target))
        .collect();
    let ordered_plugins = resolve_plugin_order::<T, dyn Plugin<T>>(enabled_plugins)?;
    let mut installed_keys = Vec::with_capacity(ordered_plugins.len());

    for plugin in ordered_plugins {
        plugin
            .install(target)
            .await
            .with_context(|| format!("安装插件失败: {}", plugin.key()))?;
        installed_keys.push(plugin.key());
    }

    Ok(installed_keys)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{Plugin, PluginFuture, plugin_key};

    use super::*;

    #[derive(Default)]
    struct TestTarget {
        enabled: bool,
        installed: Vec<&'static str>,
    }

    struct EnabledPlugin;
    struct DisabledPlugin;

    impl Plugin<TestTarget> for EnabledPlugin {
        fn order(&self) -> i32 {
            10
        }

        fn install<'a>(&'a self, target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async move {
                target.installed.push(self.key());
                Ok(())
            })
        }
    }

    impl Plugin<TestTarget> for DisabledPlugin {
        fn enabled(&self, target: &TestTarget) -> bool {
            target.enabled
        }

        fn install<'a>(&'a self, target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async move {
                target.installed.push(self.key());
                Ok(())
            })
        }
    }

    fn dynamic<P>(plugin: P) -> DynPlugin<TestTarget>
    where
        P: Plugin<TestTarget>,
    {
        Arc::new(plugin)
    }

    #[tokio::test]
    async fn installs_only_enabled_plugins() -> Result<()> {
        let mut target = TestTarget::default();
        let plugins = vec![dynamic(DisabledPlugin), dynamic(EnabledPlugin)];

        let installed = install_plugins(&mut target, plugins).await?;

        assert_eq!(installed, [plugin_key::<EnabledPlugin>()]);
        assert_eq!(target.installed, installed);
        Ok(())
    }

    #[test]
    fn rejects_rudi_name_that_differs_from_plugin_type() {
        struct WrongNameModule;

        impl rudi::Module for WrongNameModule {
            fn providers() -> Vec<rudi::DynProvider> {
                vec![
                    rudi::singleton(|_| dynamic(EnabledPlugin))
                        .name("manual-plugin-name")
                        .into(),
                ]
            }
        }

        let mut context = rudi::Context::create(rudi::modules![WrongNameModule]);

        let error = discover_plugins::<TestTarget>(&mut context)
            .err()
            .unwrap_or_else(|| anyhow::anyhow!("预期 Rudi 注册名不一致错误"));

        assert!(error.to_string().contains("注册名与插件类型标识不一致"));
    }
}
