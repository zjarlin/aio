use std::sync::Arc;

use dioxus::prelude::*;
use rudi::Singleton;
use az_dioxus_admin_shell::{
    ConventionPageContext, ConventionPageProvider, DynConventionPageProvider,
};
use studio::EndpointPage;

#[derive(Clone, Debug, Default)]
struct Page;

impl ConventionPageProvider for Page {
    fn key(&self) -> &'static str {
        module_path!()
    }

    fn render(&self, context: ConventionPageContext) -> Element {
        rsx! {
            EndpointPage { page_id: context.page.id.to_string() }
        }
    }
}

#[Singleton(name = module_path!())]
fn convention_page() -> DynConventionPageProvider {
    Arc::new(Page)
}
