use std::sync::Arc;

use dill::CatalogBuilder;
use studio::ConventionEndpointProvider;

use super::model::{EndpointFuture, EndpointRequest};
use super::service::SshService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostTargetsController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostTargetsController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("b20678bc-61b8-46fe-1606-67a2e94203a6")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_targets(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("9f593fa9-3645-8d46-5354-21db32d84b69")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostTemplatesDefaultApplyController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostTemplatesDefaultApplyController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("57fcfe52-23ac-4392-8446-51956ed98ed3")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_templates_default_apply(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostCommandsController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostCommandsController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("c86b25af-dbce-78ab-9db8-1525bc284273")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_commands(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostCollectController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostCollectController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("c84a2fda-677c-1717-3927-dfa061061b5e")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_collect(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostExecuteController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostExecuteController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("4982e318-c8f5-0a9b-4c16-47b7b07eabc8")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
        self.service.post_execute(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct PostUiActionController {
    service: Arc<dyn SshService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn endpoint_id(&self) -> &'static str {
        super::util::endpoint_id("d357a2a1-0f03-b8e6-bb0c-6a13f3391905")
    }

    fn handle(&self, request: EndpointRequest) -> EndpointFuture<'_> {
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
