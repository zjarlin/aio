//! AIO 服务端启动阶段的插件化装配状态。

use anyhow::{Context as _, Result};
use axum::Router;
use az_plugin_core::{Db, ToastyModelContribution, database::collect_toasty_models};
use rudi::Context;
use studio::{
    ConventionContractManager, ConventionEndpointIndex, FormStateExtractor, ProgramPatchAgent,
    capability::CapabilityCatalog, program_runtime::ProgramRuntime,
};

use crate::plugin_host::HostSnapshot;

/// 由应用启动器逐步装配的服务端状态。
pub struct ApplicationStartup {
    di: Context,
    models: Option<ToastyModelContribution>,
    shared_db: Option<Option<Db>>,
    native_snapshot: Option<HostSnapshot>,
    capabilities: Option<CapabilityCatalog>,
    convention_endpoints: Option<ConventionEndpointIndex>,
    program_runtime: Option<Option<ProgramRuntime>>,
    convention_contracts: Option<ConventionContractManager>,
    patch_agent: Option<ProgramPatchAgent>,
    form_state_extractor: Option<FormStateExtractor>,
    router: Router,
}

impl ApplicationStartup {
    pub fn new(mut di: Context) -> Self {
        let models = ToastyModelContribution::new(collect_toasty_models(&mut di));
        Self {
            di,
            models: Some(models),
            shared_db: None,
            native_snapshot: None,
            capabilities: None,
            convention_endpoints: None,
            program_runtime: None,
            convention_contracts: None,
            patch_agent: None,
            form_state_extractor: None,
            router: Router::new(),
        }
    }

    pub fn take_models(&mut self) -> Result<ToastyModelContribution> {
        self.models.take().context("Toasty 模型集合已被启动器消费")
    }

    pub fn di_mut(&mut self) -> &mut Context {
        &mut self.di
    }

    pub fn set_shared_db(&mut self, shared_db: Option<Db>) {
        self.shared_db = Some(shared_db);
    }

    pub fn shared_db(&self) -> Result<Option<&Db>> {
        self.shared_db
            .as_ref()
            .map(Option::as_ref)
            .context("共享数据库启动器尚未执行")
    }

    pub fn set_native_snapshot(&mut self, snapshot: HostSnapshot) {
        self.native_snapshot = Some(snapshot);
    }

    pub fn native_snapshot(&self) -> Result<&HostSnapshot> {
        self.native_snapshot
            .as_ref()
            .context("原生插件发现启动器尚未执行")
    }

    pub fn set_capabilities(&mut self, capabilities: CapabilityCatalog) {
        self.capabilities = Some(capabilities);
    }

    pub fn take_capabilities(&mut self) -> Result<CapabilityCatalog> {
        self.capabilities
            .take()
            .context("Capability 聚合启动器尚未执行")
    }

    pub fn set_convention_endpoints(&mut self, endpoints: ConventionEndpointIndex) {
        self.convention_endpoints = Some(endpoints);
    }

    pub fn take_convention_endpoints(&mut self) -> Result<ConventionEndpointIndex> {
        self.convention_endpoints
            .take()
            .context("约定接口 Provider 索引启动器尚未执行")
    }

    pub fn set_program_runtime(&mut self, runtime: Option<ProgramRuntime>) {
        self.program_runtime = Some(runtime);
    }

    pub fn program_runtime(&self) -> Result<Option<ProgramRuntime>> {
        self.program_runtime
            .as_ref()
            .cloned()
            .context("Studio ProgramRuntime 启动器尚未执行")
    }

    pub fn set_convention_contracts(&mut self, contracts: ConventionContractManager) {
        self.convention_contracts = Some(contracts);
    }

    pub fn convention_contracts(&self) -> Result<ConventionContractManager> {
        self.convention_contracts
            .clone()
            .context("约定接口路由启动器尚未执行")
    }

    pub fn set_patch_agent(&mut self, patch_agent: ProgramPatchAgent) {
        self.patch_agent = Some(patch_agent);
    }

    pub fn take_patch_agent(&mut self) -> Result<ProgramPatchAgent> {
        self.patch_agent
            .take()
            .context("ProgramPatchAgent 启动器尚未执行")
    }

    pub fn set_form_state_extractor(&mut self, extractor: FormStateExtractor) {
        self.form_state_extractor = Some(extractor);
    }

    pub fn take_form_state_extractor(&mut self) -> Result<FormStateExtractor> {
        self.form_state_extractor
            .take()
            .context("FormStateExtractor 启动器尚未执行")
    }

    pub fn merge_router(&mut self, router: Router) {
        let current = std::mem::take(&mut self.router);
        self.router = current.merge(router);
    }

    pub fn into_router(self) -> Router {
        self.router
    }
}
