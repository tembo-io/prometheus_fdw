# This image is a normal Postgres container,
# configured for local dev (network access configs and password are insecure),
# plus the CLI tool "Trunk" for installing extensions from https://pgt.dev
#
# Source: https://github.com/tembo-io/tembo-images
FROM quay.io/tembo/tembo-local:latest

RUN trunk install prometheus_fdw --version 0.1.3
