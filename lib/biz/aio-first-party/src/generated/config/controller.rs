use std::sync::Arc;

use dill::CatalogBuilder;
use studio::{ConventionEndpointFuture, ConventionEndpointProvider, ConventionEndpointRequest};

use super::contract::ConfigService;

#[dill::component]
#[dill::interface(dyn ConventionEndpointProvider)]
#[dill::scope(dill::Singleton)]
pub(crate) struct GetStatusController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetStatusController {
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
pub(crate) struct GetDotfilesController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetDotfilesController {
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
pub(crate) struct GetPairingController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetPairingController {
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
pub(crate) struct GetEntriesController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for GetEntriesController {
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
pub(crate) struct PostEntryController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for PostEntryController {
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
pub(crate) struct PostUiActionController {
    service: Arc<dyn ConfigService>,
}

impl ConventionEndpointProvider for PostUiActionController {
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
