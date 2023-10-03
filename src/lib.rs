use pgrx::warning;
use pgrx::{pg_sys, prelude::*, JsonB};
use reqwest::{self, Client};
use std::collections::HashMap;
use std::env;
use tokio::runtime::Runtime;
// use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use serde_json::Value as JsonValue;
use std::str::FromStr;
use supabase_wrappers::prelude::*;
pgrx::pg_module_magic!();

fn body_to_rows(
    resp: &JsonValue,
    obj_key: &str,
    normal_cols: Vec<(&str, &str, &str)>,
    tgt_cols: &[Column],
) -> Vec<Row> {
    let mut result = Vec::new();

    let objs = if resp.is_array() {
        // If `resp` is directly an array
        resp.as_array().unwrap()
    } else {
        // If `resp` is an object containing the array under `obj_key`
        match resp
            .as_object()
            .and_then(|v| v.get(obj_key))
            .and_then(|v| v.as_array())
        {
            Some(objs) => objs,
            None => return result,
        }
    };

    for obj in objs {
        let mut row = Row::new();

        // extract normal columns
        for tgt_col in tgt_cols {
            if let Some((src_name, col_name, col_type)) =
                normal_cols.iter().find(|(_, c, _)| c == &tgt_col.name)
            {
                // Navigate through nested properties
                let mut current_value: Option<&JsonValue> = Some(obj);
                for part in src_name.split('.') {
                    current_value = current_value.unwrap().as_object().unwrap().get(part);
                }

                if *src_name == "email_addresses" {
                    current_value = current_value
                        .and_then(|v| v.as_array().and_then(|arr| arr.get(0)))
                        .and_then(|first_obj| {
                            first_obj
                                .as_object()
                                .and_then(|obj| obj.get("email_address"))
                        });
                }

                let cell = current_value.and_then(|v| match *col_type {
                    "bool" => v.as_bool().map(Cell::Bool),
                    "i64" => v.as_i64().map(Cell::I64),
                    "string" => v.as_str().map(|a| Cell::String(a.to_owned())),
                    "timestamp" => v.as_str().map(|a| {
                        let secs = a.parse::<i64>().unwrap() / 1000;
                        let ts = to_timestamp(secs as f64);
                        Cell::Timestamp(ts.to_utc())
                    }),
                    "timestamp_iso" => v.as_str().map(|a| {
                        let ts = Timestamp::from_str(a).unwrap();
                        Cell::Timestamp(ts)
                    }),
                    "json" => Some(Cell::Json(JsonB(v.clone()))),
                    _ => None,
                });
                row.push(col_name, cell);
            }
        }

        // put all properties into 'attrs' JSON column
        if tgt_cols.iter().any(|c| &c.name == "attrs") {
            let attrs = serde_json::from_str(&obj.to_string()).unwrap();
            row.push("attrs", Some(Cell::Json(JsonB(attrs))));
        }

        result.push(row);
    }
    result
}

// convert response body text to rows
fn resp_to_rows(obj: &str, resp: &JsonValue, tgt_cols: &[Column]) -> Vec<Row> {
    let mut result = Vec::new();

    match obj {
        "metric_labels" => {
            result = body_to_rows(
                resp,
                "data",
                vec![
                    ("id", "metric_id", "i64"),
                    ("metric_name", "metric_name", "string"),
                    ("metric_name_label", "metric_name_label", "string"),
                    ("metric_labels", "metric_labels", "json"),
                ],
                tgt_cols,
            );
        }
        "metric_values" => {
            result = body_to_rows(
                resp,
                "data",
                vec![
                    ("id", "metric_id", "i64"),
                    ("timestamp", "timestamp", "i64"),
                    ("value", "value", "i64"),
                ],
                tgt_cols,
            );
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
    client: Option<Client>,
    scan_result: Option<Vec<Row>>,
    tgt_cols: Vec<Column>,
}

impl PrometheusFdw {
    const DEFAULT_BASE_URL: &'static str =
        "https://prometheus-control-1.use1.dev.plat.cdb-svc.com/";

    fn map_operator(op: &str) -> &str {
        match op {
            "=" => "=\"",
            "!=" => "!=\"",
            ">" => ">\"",
            "<" => "<\"",
            ">=" => ">=\"",
            "<=" => "<=\"",
            _ => {
                println!("unsupported operator: {}", op);
                "\""
            }
        }
    }

    fn value_to_promql_string(value: &supabase_wrappers::interface::Value) -> String {
        match value {
            supabase_wrappers::interface::Value::Cell(cell) => match cell {
                supabase_wrappers::interface::Cell::String(s) => s.clone(),
                supabase_wrappers::interface::Cell::I8(i) => i.to_string(),
                supabase_wrappers::interface::Cell::I16(i) => i.to_string(),
                supabase_wrappers::interface::Cell::I32(i) => i.to_string(),
                supabase_wrappers::interface::Cell::I64(i) => i.to_string(),
                supabase_wrappers::interface::Cell::F32(f) => f.to_string(),
                supabase_wrappers::interface::Cell::F64(f) => f.to_string(),
                supabase_wrappers::interface::Cell::Bool(b) => b.to_string(),
                supabase_wrappers::interface::Cell::Date(d) => d.to_string(),
                supabase_wrappers::interface::Cell::Timestamp(ts) => ts.to_string(),
                supabase_wrappers::interface::Cell::Json(j) => {
                    match serde_json::to_string(j) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to serialize JsonB to String: {}", e);
                            String::new() // Return an empty string on error
                        }
                    }
                }
                supabase_wrappers::interface::Cell::Numeric(n) => n.to_string(),
            },
            supabase_wrappers::interface::Value::Array(cells) => {
                // Join the string representations of the cells with commas
                cells
                    .iter()
                    .map(|cell| match cell {
                        supabase_wrappers::interface::Cell::String(s) => s.clone(),
                        supabase_wrappers::interface::Cell::I8(i) => i.to_string(),
                        supabase_wrappers::interface::Cell::I16(i) => i.to_string(),
                        supabase_wrappers::interface::Cell::I32(i) => i.to_string(),
                        supabase_wrappers::interface::Cell::I64(i) => i.to_string(),
                        supabase_wrappers::interface::Cell::F32(f) => f.to_string(),
                        supabase_wrappers::interface::Cell::F64(f) => f.to_string(),
                        supabase_wrappers::interface::Cell::Bool(b) => b.to_string(),
                        supabase_wrappers::interface::Cell::Date(d) => d.to_string(),
                        supabase_wrappers::interface::Cell::Timestamp(ts) => ts.to_string(),
                        supabase_wrappers::interface::Cell::Json(j) => {
                            match serde_json::to_string(j) {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("Failed to serialize JsonB to String: {}", e);
                                    String::new() // Return an empty string on error
                                }
                            }
                        }
                        supabase_wrappers::interface::Cell::Numeric(n) => n.to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(",")
            }
        }
    }

    fn build_url(&self, obj: &str, _options: &HashMap<String, String>, quals: &[Qual]) -> String {
        let base_url = "https://prometheus-control-1.use1.dev.plat.cdb-svc.com/api/v1/query";

        match obj {
            "metric_labels" | "metric_values" => {
                // Find the metric_name filter from quals
                let metric_name_filter = quals
                    .iter()
                    .find(|qual| qual.field == "metric_name" && qual.operator == "=");

                // If a metric_name filter is found, build the query URL
                if let Some(metric_name_qual) = metric_name_filter {
                    let metric_name = Self::value_to_promql_string(&metric_name_qual.value);
                    let ret = format!("{}?query={}", base_url, metric_name);
                    ret
                } else {
                    println!("No metric_name filter found in quals");
                    "".to_string()
                }
            }
            _ => {
                println!("unsupported object: {:#?}", obj);
                "".to_string()
            }
        }
    }

    // Helper function to map SQL operators to PromQL operators
}

impl ForeignDataWrapper for PrometheusFdw {
    fn new(_options: &HashMap<String, String>) -> Self {
        let mut ret = Self {
            rt: create_async_runtime(),
            client: None,
            tgt_cols: Vec::new(),
            scan_result: None,
        };

        // create client
        let client = reqwest::Client::new();
        ret.client = Some(client);

        warning!("created client");

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
        let api_key = "".to_string();

        if let Some(client) = &self.client {
            let mut result = Vec::new();

            let url = self.build_url("metric_labels", options, quals);
            warning!("url: {}", url);

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
