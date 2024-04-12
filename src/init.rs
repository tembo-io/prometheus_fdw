use pgrx::prelude::*;
use pgrx::spi::Spi;
use std::error::Error;

#[pg_extern]
fn create_table() -> Result<(), Box<dyn Error>> {
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

    Spi::run(queries);

    Ok(())
}

fn index_tables() {
    // Implement indexing logic
}

fn create_partitions(){
    // Implement partitioning logic
}
