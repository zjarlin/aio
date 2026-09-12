ARG BASE_IMAGE
FROM ${BASE_IMAGE}
USER root
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates git zsh clang lld pkg-config libssl-dev xz-utils && rm -rf /var/lib/apt/lists/*
ENV CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup PATH=/opt/cargo/bin:$PATH
RUN curl --connect-timeout 20 --max-time 180 --retry 3 -fsSL https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init -o /tmp/rustup-init && chmod +x /tmp/rustup-init && /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain nightly-2026-05-25 && rm /tmp/rustup-init
RUN rustup target add wasm32-unknown-unknown
RUN curl -fsSL https://github.com/DioxusLabs/dioxus/releases/download/v0.7.9/dx-x86_64-unknown-linux-gnu.tar.gz -o /tmp/dx.tar.gz && echo '3b132551b480bc96f938f9f0d37936ee1190f994977539dcc347eaf38540d005  /tmp/dx.tar.gz' | sha256sum -c - && tar -xzf /tmp/dx.tar.gz -C /usr/local/bin && rm /tmp/dx.tar.gz
RUN cargo install --locked wasm-bindgen-cli --version 0.2.128 && cargo install --locked wasm-tools --version 1.240.0
RUN curl --connect-timeout 20 --max-time 180 --retry 3 -fsSL https://github.com/WebAssembly/binaryen/releases/download/version_127/binaryen-version_127-x86_64-linux.tar.gz -o /tmp/binaryen.tar.gz && echo 'c8ebe5d00c978601086bcad33b2c80fcfe33d6a8b87b754ba4ea86a9a16cc316  /tmp/binaryen.tar.gz' | sha256sum -c - && mkdir /opt/binaryen && tar -xzf /tmp/binaryen.tar.gz --strip-components=1 -C /opt/binaryen && rm /tmp/binaryen.tar.gz
ENV PATH=/opt/binaryen/bin:$PATH
RUN curl --connect-timeout 20 --max-time 120 --retry 3 -fsSL https://registry.npmjs.org/@esbuild/linux-x64/-/linux-x64-0.27.3.tgz -o /tmp/esbuild.tgz && echo '066e20cdb882994160e18524a552b97e03648eb9aa0c7cdf5680a6493be65ab2  /tmp/esbuild.tgz' | sha256sum -c - && tar -xzf /tmp/esbuild.tgz --strip-components=2 -C /usr/local/bin package/bin/esbuild && rm /tmp/esbuild.tgz
RUN chmod a+rx /opt/cargo /opt/rustup /opt/cargo/bin && mkdir -p /cache /source && chmod 777 /cache /source
ENV HOME=/cache CARGO_HOME=/cache/cargo RUSTUP_TOOLCHAIN=nightly-2026-05-25
USER 65534:65534
