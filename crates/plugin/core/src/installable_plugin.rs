//! 可安装插件的通用契约和依赖顺序解析。

use std::{
    any::type_name,
    collections::{BTreeMap, HashMap},
    future::Future,
    marker::PhantomData,
    pin::Pin,
    sync::Arc,
};

use anyhow::{Context as _, Result, bail, ensure};

/// 插件异步安装过程。
pub type PluginFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + 'a>>;

/// 指定宿主类型的动态插件。
pub type DynPlugin<T> = Arc<dyn Plugin<T>>;

/// 返回具体插件类型的全限定名。
pub fn plugin_key<P: ?Sized + 'static>() -> &'static str {
    type_name::<P>()
}

/// 由具体类型表达的插件依赖。
pub struct PluginType<T> {
    key: &'static str,
    target: PhantomData<fn(&mut T)>,
}

impl<T> PluginType<T> {
    /// 声明当前插件依赖另一个具体插件类型。
    pub fn of<P>() -> Self
    where
        P: Plugin<T>,
    {
        Self {
            key: plugin_key::<P>(),
            target: PhantomData,
        }
    }

    pub fn key(&self) -> &'static str {
        self.key
    }
}

/// 可安装到指定宿主对象的通用插件。
pub trait Plugin<T>: Send + Sync + 'static {
    /// 默认使用实现类型的全限定名作为唯一标识。
    fn key(&self) -> &'static str {
        plugin_key::<Self>()
    }

    /// 数值越小越先执行，显式依赖始终优先于当前插件。
    fn order(&self) -> i32 {
        i32::MAX
    }

    /// 必须先于当前插件安装的具体插件类型。
    fn dependencies(&self) -> Vec<PluginType<T>> {
        Vec::new()
    }

    /// 当前宿主配置是否启用该插件。
    fn enabled(&self, _target: &T) -> bool {
        true
    }

    /// 将插件能力安装到宿主对象。
    fn install<'a>(&'a self, target: &'a mut T) -> PluginFuture<'a>;
}

/// 按依赖关系和执行顺序解析插件安装次序。
pub fn resolve_plugin_order<T: 'static, P>(mut plugins: Vec<Arc<P>>) -> Result<Vec<Arc<P>>>
where
    P: Plugin<T> + ?Sized,
{
    let mut counts = BTreeMap::new();
    for plugin in &plugins {
        *counts.entry(plugin.key()).or_insert(0_usize) += 1;
    }
    let duplicate_keys = counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect::<Vec<_>>();
    ensure!(
        duplicate_keys.is_empty(),
        "插件唯一标识重复: {}",
        duplicate_keys.join(", ")
    );

    let plugins_by_key = plugins
        .iter()
        .map(|plugin| (plugin.key(), Arc::clone(plugin)))
        .collect::<BTreeMap<_, _>>();
    for plugin in &plugins {
        let missing_dependencies = plugin
            .dependencies()
            .into_iter()
            .map(|dependency| dependency.key())
            .filter(|key| !plugins_by_key.contains_key(key))
            .collect::<Vec<_>>();
        ensure!(
            missing_dependencies.is_empty(),
            "插件 {} 缺少依赖: {}",
            plugin.key(),
            missing_dependencies.join(", ")
        );
    }

    plugins.sort_by(|left, right| {
        left.order()
            .cmp(&right.order())
            .then(left.key().cmp(right.key()))
    });
    let mut states = HashMap::new();
    let mut path = Vec::new();
    let mut ordered = Vec::with_capacity(plugins.len());
    for plugin in plugins {
        visit_plugin(
            plugin.key(),
            &plugins_by_key,
            &mut states,
            &mut path,
            &mut ordered,
        )?;
    }
    Ok(ordered)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum VisitState {
    Visiting,
    Visited,
}

fn visit_plugin<T: 'static, P>(
    key: &'static str,
    plugins_by_key: &BTreeMap<&'static str, Arc<P>>,
    states: &mut HashMap<&'static str, VisitState>,
    path: &mut Vec<&'static str>,
    ordered: &mut Vec<Arc<P>>,
) -> Result<()>
where
    P: Plugin<T> + ?Sized,
{
    match states.get(key) {
        Some(VisitState::Visited) => return Ok(()),
        Some(VisitState::Visiting) => {
            let cycle_start = path.iter().position(|item| *item == key).unwrap_or(0);
            let cycle = path[cycle_start..]
                .iter()
                .copied()
                .chain(std::iter::once(key))
                .collect::<Vec<_>>()
                .join(" -> ");
            bail!("插件存在循环依赖: {cycle}");
        }
        None => {}
    }

    let plugin = plugins_by_key
        .get(key)
        .with_context(|| format!("插件索引缺少类型: {key}"))?;
    states.insert(key, VisitState::Visiting);
    path.push(key);
    let mut dependencies = plugin.dependencies();
    dependencies.sort_by_key(PluginType::key);
    for dependency in dependencies {
        visit_plugin(dependency.key(), plugins_by_key, states, path, ordered)?;
    }
    path.pop();
    states.insert(key, VisitState::Visited);
    ordered.push(Arc::clone(plugin));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTarget;
    struct Foundation;
    struct Dependent;
    struct Last;
    struct Missing;
    struct CycleLeft;
    struct CycleRight;

    macro_rules! empty_plugin {
        ($plugin:ty, $order:expr) => {
            impl Plugin<TestTarget> for $plugin {
                fn order(&self) -> i32 {
                    $order
                }

                fn install<'a>(&'a self, _target: &'a mut TestTarget) -> PluginFuture<'a> {
                    Box::pin(async { Ok(()) })
                }
            }
        };
    }

    empty_plugin!(Foundation, 20);
    empty_plugin!(Last, 30);

    impl Plugin<TestTarget> for Dependent {
        fn order(&self) -> i32 {
            10
        }

        fn dependencies(&self) -> Vec<PluginType<TestTarget>> {
            vec![PluginType::of::<Foundation>()]
        }

        fn install<'a>(&'a self, _target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    impl Plugin<TestTarget> for Missing {
        fn dependencies(&self) -> Vec<PluginType<TestTarget>> {
            vec![PluginType::of::<Foundation>()]
        }

        fn install<'a>(&'a self, _target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    impl Plugin<TestTarget> for CycleLeft {
        fn dependencies(&self) -> Vec<PluginType<TestTarget>> {
            vec![PluginType::of::<CycleRight>()]
        }

        fn install<'a>(&'a self, _target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    impl Plugin<TestTarget> for CycleRight {
        fn dependencies(&self) -> Vec<PluginType<TestTarget>> {
            vec![PluginType::of::<CycleLeft>()]
        }

        fn install<'a>(&'a self, _target: &'a mut TestTarget) -> PluginFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    fn dynamic<P>(plugin: P) -> Arc<dyn Plugin<TestTarget>>
    where
        P: Plugin<TestTarget>,
    {
        Arc::new(plugin)
    }

    #[test]
    fn dependencies_override_numeric_order() -> Result<()> {
        let plugins = vec![dynamic(Last), dynamic(Dependent), dynamic(Foundation)];

        let ordered = resolve_plugin_order(plugins)?;
        let keys = ordered
            .into_iter()
            .map(|plugin| plugin.key())
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            [
                plugin_key::<Foundation>(),
                plugin_key::<Dependent>(),
                plugin_key::<Last>(),
            ]
        );
        Ok(())
    }

    #[test]
    fn rejects_duplicate_plugin_types() {
        let error = resolve_plugin_order(vec![dynamic(Foundation), dynamic(Foundation)])
            .err()
            .unwrap_or_else(|| anyhow::anyhow!("预期插件标识重复错误"));

        assert!(error.to_string().contains("唯一标识重复"));
    }

    #[test]
    fn rejects_missing_dependencies() {
        let error = resolve_plugin_order(vec![dynamic(Missing)])
            .err()
            .unwrap_or_else(|| anyhow::anyhow!("预期插件依赖缺失错误"));

        assert!(error.to_string().contains("缺少依赖"));
    }

    #[test]
    fn rejects_dependency_cycles() {
        let error = resolve_plugin_order(vec![dynamic(CycleLeft), dynamic(CycleRight)])
            .err()
            .unwrap_or_else(|| anyhow::anyhow!("预期插件循环依赖错误"));

        assert!(error.to_string().contains("循环依赖"));
    }
}
