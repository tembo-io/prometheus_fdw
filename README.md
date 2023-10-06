## Prometheus_fdw
NOTE: add a cron job to make this automatic
NOTE: write tests

### Pre-requisistes

- have the latest version of `prometheus_fdw` extension enabled in your instance

### Set-up 

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


Create tables to store information locally:

```
CREATE TABLE IF NOT EXISTS metrics_local (
  metric_name TEXT,
  metric_labels JSONB,
  metric_time BIGINT, 
  metric_value FLOAT8
);

-- Create metric_labels table
CREATE TABLE public.metric_labels (
    id BIGSERIAL NOT NULL,
    name TEXT NOT NULL,
    labels JSONB,
    PRIMARY KEY (id),
    UNIQUE (name, labels)
);

-- Create partitioned metric_values table
CREATE TABLE public.metric_values (
    id BIGINT NOT NULL,
    time BIGINT,
    value DOUBLE PRECISION NOT NULL
);
```

NOTE: need to index and partition the tables

## Queries

To simply run the fdw and look at values

```
select * from metrics where metric_name='container_cpu_usage_seconds_total' AND metric_time > 1696046800 AND metric_time < 1696133000;
```

To store the information in your local database for future use
```
INSERT INTO metrics_local
select * from metrics where metric_name='container_cpu_usage_seconds_total' AND metric_time > 1696046800 AND metric_time < 1696046800;
```

To save information for long term and/or analysis 
```
INSERT INTO public.metric_labels (name, labels)
SELECT 
    metric_name, 
    metric_labels
FROM metrics_local
WHERE 
    metric_time > 1696046800 AND metric_time < 1696133000
    AND metric_name = 'container_cpu_usage_seconds_total'
ON CONFLICT (name, labels) DO NOTHING;
```

To store values for long term and/or analysis

```
ALTER TABLE metric_values
ADD CONSTRAINT metric_values_unique UNIQUE (id, time);
```

```
INSERT INTO metric_values (id, time, value)
SELECT
    mlab.id,
    ml.metric_time,
    ml.metric_value
FROM
    metrics_local ml
INNER JOIN
    metric_labels mlab
ON
    ml.metric_name = mlab.name AND ml.metric_labels = mlab.labels
ON CONFLICT (id, time) DO NOTHING;
```