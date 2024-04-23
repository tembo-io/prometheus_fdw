use pgrx::prelude::*;
use std::error::Error;

#[pg_extern]
fn basic_setup(base_url: &str) -> Result<(), Box<dyn Error>> {
    let queries = format!(
        r#"
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
                base_url '{}');

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
              step '{}'
            );
        "#,
        base_url, "10m"
    );

    Spi::run(&queries);

    Ok(())
}

/// Creates the necessary tables for metric tracking.
#[pg_extern]
fn create_tables() -> Result<(), Box<dyn Error>> {
    let queries = r#"
        CREATE TABLE IF NOT EXISTS metric_labels (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            labels jsonb NOT NULL
        );
        CREATE TABLE IF NOT EXISTS metric_values (
            label_id INTEGER REFERENCES metric_labels (id),
            time TIMESTAMP NOT NULL,
            value DOUBLE PRECISION NOT NULL
        ) PARTITION BY RANGE (time);
    "#;

    Spi::run(&queries);

    Ok(())
}

/// Creates indexes to optimize query performance.
#[pg_extern]
fn create_indexes() -> Result<(), Box<dyn Error>> {
    let queries = r#"
        CREATE INDEX idx_metric_labels_name ON metric_labels (name);
        CREATE INDEX idx_metric_labels_labels ON metric_labels USING GIN (labels);
        CREATE INDEX idx_metric_values_time ON metric_values (time);
        CREATE INDEX idx_metric_values_label_id ON metric_values (label_id);
    "#;

    Spi::run(queries);
    Ok(())
}

/// Sets up partitioning for the metric_values table and configures retention policy.
#[pg_extern]
fn create_partitions(retention_period: &str) -> Result<(), Box<dyn Error>> {
    let setup_partitioning = r#"
        SELECT create_parent('public.metric_values', 'time', 'native', '1 day');
    "#;

    // Execute the partition setup query
    Spi::run(setup_partitioning);

    let setup_retention = format!(
        r#"
            UPDATE part_config
            SET retention = '{}',
                retention_keep_table = false,
                retention_keep_index = false,
                infinite_time_partitions = true
            WHERE parent_table = 'public.metric_values';
        "#,
        retention_period
    );

    // Execute the retention setup query
    Spi::run(&setup_retention);
    Ok(())
}
