# This image is a normal Postgres container,
# configured for local dev (network access configs and password are insecure),
# plus the CLI tool "Trunk" for installing extensions from https://pgt.dev
#
# Source: https://github.com/tembo-io/tembo-images
# Documentation: https://tembo.io/docs/tembo-cloud/try-extensions-locally
FROM quay.io/tembo/tembo-local:latest

# We will use pg_partman to optimize storing and querying timeseries data
RUN trunk install pg_partman --version 4.7.4
# We will use pg_cron to schedule syncing prometheus into the local DB
RUN trunk install pg_cron --version 1.5.2

RUN trunk install prometheus_fdw --version 0.1.3

# Extra Postgres configurations
COPY custom.conf $PGDATA/extra-configs
