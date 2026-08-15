use dioxus::prelude::*;
use studio::SymbolId;

mod aio_first_party_algorithms;
mod aio_first_party_assets;
mod aio_first_party_config;
mod aio_first_party_drive;
mod aio_first_party_gateway;
mod aio_first_party_linux;
mod aio_first_party_software;

pub fn render(page_id: SymbolId) -> Element {
    match page_id.to_string().as_str() {
        "d757953c-7960-e59a-a040-14acddf59b15" => aio_first_party_config::render(),
        "9606cfae-2c43-f884-2a4c-a0ad01b36b49" => aio_first_party_assets::render(),
        "a2b391ae-2c26-f31f-93a8-348c5dfc1341" => aio_first_party_drive::render(),
        "545103ac-56a3-5c43-a15c-d1db7263afae" => aio_first_party_software::render(),
        "f355b92d-cda9-b63c-06d7-4e44f0c4735c" => aio_first_party_algorithms::render(),
        "d379940c-a697-5f35-7673-c1565c252c2d" => aio_first_party_gateway::render(),
        "c080eb9d-00ee-3bcb-7219-4f2b3a5ed84d" => aio_first_party_linux::render(),
        _ => rsx! { studio::EndpointPage { page_id: page_id.to_string() } },
    }
}
