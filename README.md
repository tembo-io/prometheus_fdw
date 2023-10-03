## Prometheus_fdw

### Pre-requisistes

- have the v0.2.2 of `prometheus_fdw` extension enabled in your instance

`create extension prometheus_fdw;`

Create the foreign data wrapper:

```
create foreign data wrapper prometheus_wrapper
  handler prometheus_fdw_handler
  validator prometheus_fdw_validator;
```

```
create server my_prometheus_server
  foreign data wrapper prometheus_wrapper;
```

Create Foreign Table:

### Metric Labels Table

```
CREATE FOREIGN TABLE IF NOT EXISTS metric_labels (
  metric_id BIGINT,
  metric_name TEXT NOT NULL,
  metric_name_label TEXT NOT NULL,
  metric_labels jsonb
)
SERVER my_prometheus_server
OPTIONS (
  object 'metric_labels'
);
```

### Metrics Value Table

NOTE: NEED TO ADD PARTIONTION TO THIS TABLE

```
CREATE FOREIGN TABLE IF NOT EXISTS metric_values (
  metric_id BIGINT, 
  metric_time TIMESTAMPTZ, 
  metric_value FLOAT8 
  ) 
server my_prometheus_server
options (
  object 'metric_values'
);
```
