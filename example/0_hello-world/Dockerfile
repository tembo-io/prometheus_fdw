# This image is a normal Postgres container,
# configured for local dev (network access configs and password are insecure),
# plus the CLI tool "Trunk" for installing extensions from https://pgt.dev
#
# Source: https://github.com/tembo-io/tembo-images
FROM quay.io/tembo/tembo-local:latest

ENV PROMETHEUS_BASE_URL="https://prometheus-data-1.use1.dev.plat.cdb-svc.com"
RUN trunk install prometheus_fdw
