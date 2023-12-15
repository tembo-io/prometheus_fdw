-- Table for saving results from FDW
CREATE TABLE metrics_local (
  metric_name TEXT,
  metric_labels JSONB,
  metric_time BIGINT,
  metric_value FLOAT8
);

-- Create metric_labels table
CREATE TABLE metric_labels (
    id BIGSERIAL NOT NULL,
    name TEXT NOT NULL,
    labels JSONB,
    PRIMARY KEY (id),
    UNIQUE (name, labels)
);

-- Create metric_values table
CREATE TABLE metric_values (
    label_id bigint NOT NULL,
    "time" bigint NOT NULL,
    value double precision NOT NULL,
    id serial NOT NULL,
    PRIMARY KEY (id, "time"),
    UNIQUE (label_id, "time"),
    FOREIGN KEY (label_id) REFERENCES public.metric_labels(id)
) PARTITION BY RANGE ("time");

-- Configure automatic partitioning
SELECT create_parent(
    p_parent_table := 'public.metric_values',
    p_control := 'time',
    p_type := 'native',
    p_interval := 'daily',
    p_automatic_maintenance := 'on',
    p_start_partition := '2023-09-06',
    p_epoch := 'seconds',
    p_premake := 30
);
-- Configure retention
UPDATE part_config
    SET retention = '30 days',
        retention_keep_table = false,
        retention_keep_index = false,
        infinite_time_partitions = true
    WHERE parent_table = 'public.metric_values';

CREATE INDEX IF NOT EXISTS metric_labels_labels_idx ON metric_labels USING GIN (labels);
CREATE INDEX IF NOT EXISTS metric_values_id_time_idx on metric_values USING btree (id, time DESC);
CREATE INDEX IF NOT EXISTS metric_values_time_idx  on metric_values USING btree (time DESC);
