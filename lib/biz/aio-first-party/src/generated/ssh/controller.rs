use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::SshService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostTargetsController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostTargetsController {
    fn comment(&self) -> &'static str {
        "保存 SSH 目标"
    }

    fn endpoint_id(&self) -> &'static str {
        "b20678bc-61b8-46fe-1606-67a2e94203a6"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_targets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "SSH 运维状态"
    }

    fn endpoint_id(&self) -> &'static str {
        "9f593fa9-3645-8d46-5354-21db32d84b69"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostTemplatesDefaultApplyController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostTemplatesDefaultApplyController {
    fn comment(&self) -> &'static str {
        "初始化 SSH 低代码模板"
    }

    fn endpoint_id(&self) -> &'static str {
        "57fcfe52-23ac-4392-8446-51956ed98ed3"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_templates_default_apply(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostCommandsController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostCommandsController {
    fn comment(&self) -> &'static str {
        "保存 SSH 命令"
    }

    fn endpoint_id(&self) -> &'static str {
        "c86b25af-dbce-78ab-9db8-1525bc284273"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_commands(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostCollectController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostCollectController {
    fn comment(&self) -> &'static str {
        "采集目标监测项"
    }

    fn endpoint_id(&self) -> &'static str {
        "c84a2fda-677c-1717-3927-dfa061061b5e"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_collect(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostExecuteController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostExecuteController {
    fn comment(&self) -> &'static str {
        "执行指定 SSH 命令"
    }

    fn endpoint_id(&self) -> &'static str {
        "4982e318-c8f5-0a9b-4c16-47b7b07eabc8"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_execute(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostUiActionController {
    #[debug(skip)]
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn comment(&self) -> &'static str {
        "SSH 页面操作"
    }

    fn endpoint_id(&self) -> &'static str {
        "d357a2a1-0f03-b8e6-bb0c-6a13f3391905"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_ui_action(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<PostTargetsController>();
    builder.add::<GetStatusController>();
    builder.add::<PostTemplatesDefaultApplyController>();
    builder.add::<PostCommandsController>();
    builder.add::<PostCollectController>();
    builder.add::<PostExecuteController>();
    builder.add::<PostUiActionController>();
}
