FROM rustlang/rust:nightly-bookworm AS builder
WORKDIR /workspace
RUN cargo install dioxus-cli --version 0.7.9 --locked
COPY . .
RUN dx build --package az-app-aio-first-party --platform web --release --features web --debug-symbols false
RUN cargo build -p az-app-aio-first-party --release --no-default-features --features server

FROM debian:bookworm-slim
WORKDIR /opt/aio
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /workspace/target/release/az-app-aio-first-party /usr/local/bin/az-app-aio-first-party
COPY --from=builder /workspace/target/dx/az-app-aio-first-party/release/web/public /opt/aio/web
ENV AZ_AIO_WEB_DIST=/opt/aio/web
EXPOSE 8080
CMD ["/usr/local/bin/az-app-aio-first-party"]
