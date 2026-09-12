ARG BASE_IMAGE
FROM ${BASE_IMAGE}
USER root
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates git unzip xz-utils libatomic1 && rm -rf /var/lib/apt/lists/*
RUN curl --connect-timeout 20 --max-time 300 --retry 3 -fsSL https://packages.jetbrains.team/maven/p/amper/amper/org/jetbrains/kotlin/kotlin-cli/0.12.0-dev-4233/kotlin-cli-0.12.0-dev-4233-dist.tgz -o /tmp/kotlin-cli.tgz && echo 'e4a266c00bb6893401dd315772e5a0b1f6550e6a2aa7cb08715c5651e8e9a9f0  /tmp/kotlin-cli.tgz' | sha256sum -c - && mkdir -p /opt/kotlin-bootstrap/kotlin-cli-0.12.0-dev-4233 && tar -xf /tmp/kotlin-cli.tgz -C /opt/kotlin-bootstrap/kotlin-cli-0.12.0-dev-4233 && echo e4a266c00bb6893401dd315772e5a0b1f6550e6a2aa7cb08715c5651e8e9a9f0 > /opt/kotlin-bootstrap/kotlin-cli-0.12.0-dev-4233/.flag && rm /tmp/kotlin-cli.tgz
RUN mkdir /opt/aio-kotlin-downloads && curl --connect-timeout 20 --max-time 300 --retry 3 -fsSL https://github.com/pnpm/pnpm/releases/download/v11.9.0/pnpm-linux-x64.tar.gz -o /opt/aio-kotlin-downloads/d30df19002-pnpm-linux-x64.tar.gz && echo '8d987d82585453bcf260ea5d0bae346d298f1a5e71bcde8d99976521408ad895  /opt/aio-kotlin-downloads/d30df19002-pnpm-linux-x64.tar.gz' | sha256sum -c -
RUN curl --connect-timeout 20 --max-time 300 --retry 3 -fsSL https://nodejs.org/dist/v26.5.1/node-v26.5.1-linux-x64.tar.gz -o /opt/aio-kotlin-downloads/daf195adbe-node-v26.5.1-linux-x64.tar.gz && echo '2b07f09c218d473a26442bff5a90151f53f7b7c0a23bad244eda2c26303a2ba7  /opt/aio-kotlin-downloads/daf195adbe-node-v26.5.1-linux-x64.tar.gz' | sha256sum -c -
COPY kotlin-entrypoint.sh /usr/local/bin/aio-kotlin-entrypoint
RUN chmod 755 /usr/local/bin/aio-kotlin-entrypoint
RUN mkdir -p /cache /source && chmod 777 /cache /source
ENV HOME=/cache KOTLIN_CLI_NO_WELCOME_BANNER=1 KOTLIN_CLI_BOOTSTRAP_CACHE_DIR=/opt/kotlin-bootstrap
ENTRYPOINT ["/usr/local/bin/aio-kotlin-entrypoint"]
USER 65534:65534
