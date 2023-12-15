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
        INSERT INTO metric_values (label_id, time, value)
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


SELECT cron.schedule(
    '0 * * * *',
    $$
    SELECT
        insert_metrics(),
        insert_metric_labels(),
        insert_metric_values(),
        truncate_metrics_local();
    $$
);
