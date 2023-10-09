## Prometheus_fdw

### Pre-requisistes

- have the latest version of `prometheus_fdw` extension enabled in your instance.
- have `pg_partman` and `pg_cron` installed and enabled.

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
    label_id bigint NOT NULL,
    "time" bigint NOT NULL,
    value double precision NOT NULL,
    id serial NOT NULL,
    PRIMARY KEY (id, "time"),
    UNIQUE (label_id, "time"),
    FOREIGN KEY (label_id) REFERENCES public.metric_labels(id)
) PARTITION BY RANGE ("time");
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
  AND metric_time > 1696046800 AND metric_time < 1696133000;
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
To create partitions 30 days into the past and future:
```
SELECT create_parent(
    p_parent_table := 'public.metric_values',
    p_control := 'time',
    p_type := 'native',
    p_interval := 'daily',
    p_automatic_maintenance := 'on',
    p_start_partition := '2023-09-06', -- Adjust the date to 30 days in the past from the current date
    p_epoch := 'seconds',
    p_premake := 30 -- Create partitions 30 days into the future
);
```

To create new partitions and delete old ones automatically, set up pg_cron
```
SELECT cron.schedule('0 3 * * *', $$CALL partman.run_maintenance()$$);
```

You can manually set the data retention date 
```
UPDATE public.part_config
SET retention = '30 days'
WHERE parent_table = 'public.metric_values';
```

### Queries using pg_cron

To continuosly update the tables and information

Create functions to do the tasks mentioned above
```
CREATE OR REPLACE FUNCTION insert_metrics() RETURNS void LANGUAGE plpgsql AS $$
DECLARE
    start_time BIGINT;
    end_time BIGINT;
BEGIN
    start_time := EXTRACT(epoch FROM now() - interval '1 hour' + interval '1 second')::BIGINT;
    end_time := EXTRACT(epoch FROM now())::BIGINT;
    
    EXECUTE format(
        'INSERT INTO metrics_local
        SELECT * FROM metrics
        WHERE
          metric_name = ''container_cpu_usage_seconds_total''
          AND metric_time > %s
          AND metric_time < %s;',
        start_time,
        end_time
    );
END;
$$;

CREATE OR REPLACE FUNCTION insert_metric_labels() RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    EXECUTE '
        INSERT INTO public.metric_labels (name, labels)
        SELECT 
            metric_name, 
            metric_labels
        FROM metrics_local
        ON CONFLICT (name, labels) DO NOTHING;
    ';
END;
$$;

CREATE OR REPLACE FUNCTION insert_metric_values() RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    EXECUTE '
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
    ';
END;
$$;

CREATE OR REPLACE FUNCTION truncate_metrics_local() RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    EXECUTE 'TRUNCATE TABLE metrics_local;';
END;
$$;
```

Create a cron job to run the functions every hour
```
SELECT cron.schedule(
    '10 * * * *',
    $$
    SELECT
        insert_metrics(),
        insert_metric_labels(),
        insert_metric_values(),
        truncate_metrics_local();
    $$
);
```

NOTE: Run this command after setting every cron job
```
UPDATE cron.job SET nodename = '';
```

### Optional
 
To collect information about multiple metrics, define the insert_metrics function as follows:
```
CREATE OR REPLACE FUNCTION public.insert_metrics()
RETURNS void
LANGUAGE plpgsql
AS $function$
DECLARE
    start_time BIGINT;
    end_time BIGINT;
    metric_name text;
    metric_names text[] := ARRAY['container_cpu_usage_seconds_total', 'container_memory_working_set_bytes']; -- Add your metric names here
BEGIN
    start_time := EXTRACT(epoch FROM now() - interval '1 hour' + interval '1 second')::BIGINT;
    end_time := EXTRACT(epoch FROM now())::BIGINT;

    FOREACH metric_name IN ARRAY metric_names
    LOOP
        EXECUTE format(
            'INSERT INTO metrics_local
            SELECT * FROM metrics
            WHERE
              metric_name = %L
              AND metric_time > %s
              AND metric_time < %s;',
            metric_name,
            start_time,
            end_time
        );
    END LOOP;
END;
$function$;
```