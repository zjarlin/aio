ARG BASE_IMAGE
FROM ${BASE_IMAGE}
USER root
ENV COREPACK_HOME=/opt/corepack
RUN corepack enable && corepack prepare pnpm@10.33.2 --activate && chmod -R a+rX /opt/corepack
RUN mkdir -p /cache /source && chmod 777 /cache /source
ENV HOME=/cache
USER 65534:65534
