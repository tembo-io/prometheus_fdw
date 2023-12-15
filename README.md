## Prometheus_fdw

### Pre-requisistes

- Install `prometheus_fdw`
- (Optional) install `pg_partman` and `pg_cron`

### Quick start

`create extension prometheus_fdw;`

Create the foreign data wrapper:

```
create foreign data wrapper prometheus_wrapper
  handler prometheus_fdw_handler
  validator prometheus_fdw_validator;
```

Create the server:

```
create server my_prometheus_server
  foreign data wrapper prometheus_wrapper
  options (
    base_url '<base prometheus url>');
```

Create Foreign Table:

```
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

```
SELECT
  *
FROM metrics
WHERE
  metric_name='container_cpu_usage_seconds_total'
  AND metric_time > 1696046800 AND metric_time < 1696133000;
```

## Examples

Please see the `examples/` directory to find a basic example and a practical example. In the practical example, metrics are automatically synced into the database using `pg_cron`, and automatically expired using `pg_partman`. Performance is optimized using indexes and partitioning.
