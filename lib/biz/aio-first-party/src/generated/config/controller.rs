use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::ConfigService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetStatusController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetStatusController {
    fn comment(&self) -> &'static str {
        "Config Center Status"
    }

    fn endpoint_id(&self) -> &'static str {
        "f210d29c-eca2-993f-31b8-b4c91e0de229"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_status(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetDotfilesController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetDotfilesController {
    fn comment(&self) -> &'static str {
        "Dotfiles Status"
    }

    fn endpoint_id(&self) -> &'static str {
        "2fbe4186-c333-5c78-7ee9-66c5bc2737a7"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_dotfiles(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetPairingController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetPairingController {
    fn comment(&self) -> &'static str {
        "Pairing Identity"
    }

    fn endpoint_id(&self) -> &'static str {
        "4e007025-2296-b4ee-c7f4-d3ab00af574e"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_pairing(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct GetEntriesController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetEntriesController {
    fn comment(&self) -> &'static str {
        "Config Entries"
    }

    fn endpoint_id(&self) -> &'static str {
        "77ec27e2-49c4-c79a-57e3-8049d500866c"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.get_entries(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostEntryController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for PostEntryController {
    fn comment(&self) -> &'static str {
        "Save Config Entry"
    }

    fn endpoint_id(&self) -> &'static str {
        "eb0e4d1f-70d5-2dcb-d6a5-7820e5d6efd3"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_entry(request)
    }
}

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
#[derive(derive_more::Debug)]
pub(crate) struct PostUiActionController {
    #[debug(skip)]
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for PostUiActionController {
    fn comment(&self) -> &'static str {
        "配置页面操作"
    }

    fn endpoint_id(&self) -> &'static str {
        "9d6f0625-8f13-b23b-d6fb-b2c6053cc9e6"
    }

    fn handle(&self, request: ConventionEndpointRequest) -> ConventionEndpointFuture<'_> {
        self.service.post_ui_action(request)
    }
}

pub(crate) fn register(builder: &mut CatalogBuilder) {
    builder.add::<GetStatusController>();
    builder.add::<GetDotfilesController>();
    builder.add::<GetPairingController>();
    builder.add::<GetEntriesController>();
    builder.add::<PostEntryController>();
    builder.add::<PostUiActionController>();
}
