-- Enable the extensions
CREATE EXTENSION IF NOT EXISTS prometheus_fdw CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_partman CASCADE;
CREATE EXTENSION IF NOT EXISTS pg_cron CASCADE;

-- Create the FDW
CREATE FOREIGN DATA WRAPPER prometheus_wrapper
  HANDLER prometheus_fdw_handler
    VALIDATOR prometheus_fdw_validator;

-- Configure connection to server
CREATE SERVER my_prometheus_server
  FOREIGN DATA WRAPPER prometheus_wrapper
  OPTIONS (
    base_url 'https://prometheus-data-1.use1.dev.plat.cdb-svc.com/');

-- Create FDW table we can query to get metrics
CREATE FOREIGN TABLE metrics (
  metric_name TEXT,
  metric_labels JSONB,
  metric_time BIGINT,
  metric_value FLOAT8
  )
SERVER my_prometheus_server
OPTIONS (
  object 'metrics',
  step '10m'
);
