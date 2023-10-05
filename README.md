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
  metric_time BIGINT, 
  metric_value FLOAT8 
  ) 
server my_prometheus_server
options (
  object 'metric_values'
);
```

CREATE FOREIGN TABLE IF NOT EXISTS metrics (
  metric_name TEXT,
  metric_labels JSONB,
  metric_time BIGINT, 
  metric_value FLOAT8
  ) 
server my_prometheus_server
options (
  object 'metrics'
);


-- Create metric_labels table
CREATE TABLE public.metric_labels_local (
    metric_id BIGINT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_name_label TEXT NOT NULL,
    metric_labels JSONB,
    PRIMARY KEY (metric_id),
    UNIQUE (metric_name, metric_labels)
);

-- Create indexes for metric_labels table
CREATE INDEX metric_labels_labels_idx ON public.metric_labels USING gin (metric_labels);

-- Create partitioned metric_values_local table
CREATE TABLE public.metric_values_local (
    metric_id BIGINT NOT NULL,
    metric_time BIGINT,
    metric_value DOUBLE PRECISION NOT NULL
) PARTITION BY RANGE (metric_time);

-- Create indexes for metric_values table
CREATE INDEX metric_values_id_time_idx ON public.metric_values (metric_id, metric_time DESC);
CREATE INDEX metric_values_time_idx ON public.metric_values (metric_time DESC);

-- You can create a partition of metric_values table for a specific date range like so:
CREATE TABLE public.metric_values_20231002 PARTITION OF public.metric_values
    FOR VALUES FROM ('2023-10-02 00:00:00+00') TO ('2023-10-03 00:00:00+00');




SELECT * FROM metric_values WHERE metric_time > 1696046400 AND metric_time < 1696132800;

SELECT 
    label.metric_name AS metric_label,
    value.metric_time,
    value.metric_value
FROM 
    metric_labels AS label
JOIN 
    metric_values AS value
ON 
    label.metric_id = value.metric_id
WHERE 
    label.metric_name = 'container_threads' AND
    value.metric_time < 1696046400 AND value.metric_time > 1696132800;