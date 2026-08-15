use dioxus::prelude::*;

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! {
        document::Link {
            rel: "icon",
            r#type: "image/svg+xml",
            href: "/assets/favicon.svg?v=2026.8.11",
        }
        document::Link {
            rel: "icon",
            r#type: "image/png",
            sizes: "32x32",
            href: "/assets/favicon-32.png?v=2026.8.11",
        }
        document::Link {
            rel: "shortcut icon",
            r#type: "image/x-icon",
            href: "/assets/favicon.ico?v=2026.8.11",
        }
        document::Link {
            rel: "apple-touch-icon",
            sizes: "180x180",
            href: "/assets/apple-touch-icon.png?v=2026.8.11",
        }
        studio::PublishedApplication {
            render_convention: studio::ConventionPageRenderer::endpoint_page(),
            admin_enabled: true,
            user: studio::ApplicationUser {
                label: "zjarlin".to_owned(),
                handle: "@zjarlin".to_owned(),
                initials: "ZJ".to_owned(),
            },
        }
    }
}
