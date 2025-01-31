## Prometheus_fdw

Prometheus_fdw is an integration of Prometheus monitoring data into Postgres. It enables querying for Prometheus metrics directly within Postgres, bridging the gap between Prometheus monitoring and Postgres's robust database capabilities.

Learn more about it in this [blog post](https://tembo.io/blog/monitoring-with-prometheus-fdw). 
Watch it in action in this [demo video](https://youtu.be/LVuH4RtNQss)

[![Tembo Cloud Try Free](https://tembo.io/tryFreeButton.svg)](https://cloud.tembo.io/sign-up)

[![Static Badge](https://img.shields.io/badge/%40tembo-community?logo=slack&label=slack)](https://join.slack.com/t/tembocommunity/shared_invite/zt-20dtnhcmo-pLNV7_Aobi50TdTLpfQ~EQ)
[![PGXN version](https://badge.fury.io/pg/prometheus_fdw.svg)](https://pgxn.org/dist/prometheus_fdw/)

### Pre-requisistes

- Install `prometheus_fdw`
- (Optional) install `pg_partman` and `pg_cron`

### Quick start

`create extension prometheus_fdw;`

Create the foreign data wrapper:

```sql
create foreign data wrapper prometheus_wrapper
  handler prometheus_fdw_handler
  validator prometheus_fdw_validator;
```

Create the server:

```sql
create server my_prometheus_server
  foreign data wrapper prometheus_wrapper
  options (
    base_url '<base prometheus url>');
```

Create Foreign Table:

```sql
CREATE FOREIGN TABLE IF NOT EXISTS metrics (
  metric_name TEXT,
  metric_labels JSONB,
  metric_time BIGINT,
  metric_value FLOAT8
  )
server my_prometheus_server
options (
  object 'metrics',
  step '10m'
);
```

## Queries

To simply run the fdw and look at values

```sql
SELECT
  *
FROM metrics
WHERE
  metric_name='container_cpu_usage_seconds_total'
  AND metric_time > 1696046800 AND metric_time < 1696133000;
```

## Examples

Please see the `examples/` directory to find a basic example and a practical example. In the practical example, metrics are automatically synced into the database using `pg_cron`, and automatically expired using `pg_partman`. Performance is optimized using indexes and partitioning.
