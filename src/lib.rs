use pgrx::warning;
use pgrx::{pg_sys, prelude::*, JsonB};
use reqwest::{self, Client};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::env;
use supabase_wrappers::prelude::*;
use tokio::runtime::Runtime;
pgrx::pg_module_magic!();

// convert response body text to rows
fn resp_to_rows(obj: &str, resp: &JsonValue) -> Vec<Row> {
    let mut result = Vec::new();

    match obj {
        "metrics" => {
            if let Some(result_array) = resp["data"]["result"].as_array() {
                for result_obj in result_array {
                    let metric_name = result_obj["metric"]["__name__"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let metric_labels = result_obj["metric"].clone();
                    if let Some(values_array) = result_obj["values"].as_array() {
                        for value_pair in values_array {
                            if let (Some(time_str), Some(value_str)) =
                                (value_pair[0].as_i64(), value_pair[1].as_str())
                            {
                                if let (metric_time, Ok(metric_value)) =
                                    (time_str, value_str.parse::<f64>())
                                {
                                    let mut row = Row::new();
                                    row.push(
                                        "metric_name",
                                        Some(Cell::String(metric_name.clone())),
                                    );
                                    row.push(
                                        "metric_labels",
                                        Some(Cell::Json(JsonB(metric_labels.clone()))),
                                    );
                                    row.push("metric_time", Some(Cell::I64(metric_time)));
                                    row.push("metric_value", Some(Cell::F64(metric_value)));
                                    result.push(row);
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {
            warning!("unsupported object: {}", obj);
        }
    }

    result
}

#[wrappers_fdw(
    version = "0.0.0",
    author = "Jay Kothari",
    website = "https://tembo.io"
)]

pub(crate) struct PrometheusFdw {
    rt: Runtime,
    base_url: Option<String>,
    client: Option<Client>,
    scan_result: Option<Vec<Row>>,
    tgt_cols: Vec<Column>,
}

impl PrometheusFdw {
    fn value_to_promql_string(value: &supabase_wrappers::interface::Value) -> String {
        match value {
            supabase_wrappers::interface::Value::Cell(cell) => match cell {
                supabase_wrappers::interface::Cell::String(s) => s.clone(),
                supabase_wrappers::interface::Cell::I32(i) => i.to_string(),
                _ => {
                    println!("Unsupported cell: {:?}", cell);
                    String::new()
                }
            },
            _ => {
                println!("Unsupported value: {:?}", value);
                String::new()
            }
        }
    }

    fn build_url(&self, obj: &str, options: &HashMap<String, String>, quals: &[Qual]) -> String {
        let step = if let Some(step_value) = options.get("step") {
            step_value.to_owned()
        } else {
            warning!("Using default value of 10m for step");
            let step_value = "10m".to_string();
            step_value
        };
        match obj {
            "metrics" => {
                let metric_name_filter = quals
                    .iter()
                    .find(|qual| qual.field == "metric_name" && qual.operator == "=");
                let lower_timestamp = quals
                    .iter()
                    .find(|qual| qual.field == "metric_time" && qual.operator == ">");

                let upper_timestamp = quals
                    .iter()
                    .find(|qual| qual.field == "metric_time" && qual.operator == "<");

                if let (Some(metric_name), Some(lower_timestamp), Some(upper_timestamp)) =
                    (metric_name_filter, lower_timestamp, upper_timestamp)
                {
                    let metric_name = Self::value_to_promql_string(&metric_name.value);
                    let lower_timestamp = Self::value_to_promql_string(&lower_timestamp.value);
                    let upper_timestamp = Self::value_to_promql_string(&upper_timestamp.value);
                    let ret = format!(
                        "{}/api/v1/query_range?query={}&start={}&end={}&step={}",
                        self.base_url.as_ref().unwrap(),
                        metric_name,
                        lower_timestamp,
                        upper_timestamp,
                        step
                    );
                    ret
                } else {
                    println!("filters not found in quals");
                    "".to_string()
                }
            }
            _ => {
                println!("Unsupported object: {}", obj);
                "".to_string()
            }
        }
    }
}

impl ForeignDataWrapper for PrometheusFdw {
    fn new(options: &HashMap<String, String>) -> Self {
        let mut ret = Self {
            rt: create_async_runtime(),
            base_url: None,
            client: None,
            tgt_cols: Vec::new(),
            scan_result: None,
        };

        let base_url = if let Some(prom_url) = options.get("base_url") {
            prom_url.to_owned()
        } else {
            warning!("Cannot find prometheus base url in options");
            let prom_url = env::var("PROMETHEUS_BASE_URL").unwrap();
            prom_url
        };

        ret.base_url = Some(base_url);
        ret.client = Some(reqwest::Client::new());

        ret
    }

    fn begin_scan(
        &mut self,
        quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        _limit: &Option<Limit>,
        options: &HashMap<String, String>,
    ) {
        let obj = match require_option("object", options) {
            Some(obj) => obj,
            None => return,
        };

        self.scan_result = None;
        self.tgt_cols = columns.to_vec();

        if let Some(client) = &self.client {
            let mut result = Vec::new();

            if obj == "metrics" {
                let url = self.build_url(&obj, options, quals);

                let resp = self.rt.block_on(async { client.get(&url).send().await });

                match resp {
                    Ok(resp) => {
                        let body = self.rt.block_on(async { resp.text().await });
                        match body {
                            Ok(body) => {
                                let json: JsonValue = serde_json::from_str(&body).unwrap();
                                result = resp_to_rows(&obj, &json);
                            }
                            Err(e) => {
                                warning!("failed to get body: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warning!("failed to get response: {}", e);
                    }
                }
            }

            self.scan_result = Some(result);
        }
    }

    fn iter_scan(&mut self, row: &mut Row) -> Option<()> {
        if let Some(ref mut result) = self.scan_result {
            if !result.is_empty() {
                return result
                    .drain(0..1)
                    .last()
                    .map(|src_row| row.replace_with(src_row));
            }
        }
        None
    }

    fn end_scan(&mut self) {
        self.scan_result.take();
    }

    fn validator(options: Vec<Option<String>>, catalog: Option<pg_sys::Oid>) {
        if let Some(oid) = catalog {
            if oid == FOREIGN_TABLE_RELATION_ID {
                check_options_contain(&options, "object");
            }
        }
    }
}
