use std::sync::Arc;

use dioxus::prelude::*;
use rudi::Singleton;
use studio::{
    ConventionPageContext, ConventionPageProvider, DynConventionPageProvider,
};

#[derive(Clone, Debug, Default)]
struct Page;

impl ConventionPageProvider for Page {
    fn key(&self) -> &'static str {
        module_path!()
    }

    fn render(&self, context: ConventionPageContext) -> Element {
        rsx! {
            section { class: "p-6",
                h2 { class: "text-lg font-semibold", "{context.page.title}" }
            }
        }
    }
}

#[Singleton(name = module_path!())]
fn convention_page() -> DynConventionPageProvider {
    Arc::new(Page)
}
