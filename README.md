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

-- Create metric_values table
CREATE TABLE public.metric_values (
    id BIGINT NOT NULL,
    time BIGINT,
    value DOUBLE PRECISION NOT NULL,
    CONSTRAINT metric_values_unique UNIQUE (id, time)
) PARTITION BY RANGE (time);
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

To store the information in your local database for future use
```
INSERT INTO metrics_local
SELECT * FROM metrics 
WHERE 
  metric_name='container_cpu_usage_seconds_total' 
  AND metric_time > 1696046800 AND metric_time < 1696046800;
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
    ml.metric_labels = mlab.labels
ON CONFLICT (id, time) DO NOTHING;
```

### Performance improvements:

#### Indexing:

```
CREATE INDEX IF NOT EXISTS metric_labels_labels_idx ON metric_labels USING GIN (labels);

CREATE INDEX IF NOT EXISTS metric_values_id_time_idx on metric_values USING btree (id, time DESC); 

CREATE INDEX IF NOT EXISTS metric_values_time_idx  on metric_values USING btree (time DESC);
```

#### Partioning:
This script creates partitions for the past 30 days
```
DO $$
DECLARE
    day_offset INT;
    day_start BIGINT;
    day_end BIGINT;
BEGIN
    -- Adjust the generate_series values to fit your desired range of dates.
    -- Here it's set to create partitions for 30 days from 30 days ago to yesterday.
    FOR day_offset IN SELECT generate_series(0, 29) AS day_num LOOP
        day_start := EXTRACT(EPOCH FROM date_trunc('day', current_date - day_offset))::BIGINT;
        day_end := EXTRACT(EPOCH FROM (date_trunc('day', current_date - day_offset) + interval '1 day') - interval '1 second')::BIGINT;
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS metric_values_%s PARTITION OF metric_values FOR VALUES FROM (%s) TO (%s)',
            TO_CHAR(current_date - day_offset, 'YYYY_MM_DD'),
            day_start,
            day_end
        );
    END LOOP;
END $$;
```

To drop partitions:
```
DO $$
  DECLARE
      r RECORD;
  BEGIN
      FOR r IN (
          SELECT table_name
          FROM information_schema.tables
          WHERE table_name LIKE 'metric_values_%'
          AND table_schema = 'public'  -- Adjust schema name if necessary
      ) LOOP
          EXECUTE format('DROP TABLE %I CASCADE', r.table_name);
      END LOOP;
  END $$;
```