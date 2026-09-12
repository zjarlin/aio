ARG BASE_IMAGE
FROM ${BASE_IMAGE}
USER root
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates git unzip xz-utils && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /cache /source && chmod 777 /cache /source
ENV HOME=/cache KOTLIN_CLI_NO_WELCOME_BANNER=1
USER 65534:65534
