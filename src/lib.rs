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
        "users" => {
            result = body_to_rows(
                resp,
                "data",
                vec![
                    ("id", "user_id", "string"),
                    ("first_name", "first_name", "string"),
                    ("last_name", "last_name", "string"),
                    ("email_addresses", "email", "string"),
                    ("gender", "gender", "string"),
                    ("created_at", "created_at", "i64"),
                    ("updated_at", "updated_at", "i64"),
                    ("last_sign_in_at", "last_sign_in_at", "i64"),
                    ("phone_numbers", "phone_numbers", "i64"),
                    ("username", "username", "string"),
                ],
                tgt_cols,
            );
        }
        "organizations" => {
            result = body_to_rows(
                resp,
                "data",
                vec![
                    ("id", "organization_id", "string"),
                    ("name", "name", "string"),
                    ("slug", "slug", "string"),
                    ("created_at", "created_at", "i64"),
                    ("updated_at", "updated_at", "i64"),
                    ("created_by", "created_by", "string"),
                ],
                tgt_cols,
            );
        }
        "organization_memberships" => {
            result = body_to_rows(
                resp,
                "data",
                vec![
                    ("public_user_data.user_id", "user_id", "string"),
                    ("organization.id", "organization_id", "string"),
                    ("role", "role", "string"),
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
    version = "0.2.4",
    author = "Jay Kothari",
    website = "https://tembo.io"
)]

pub(crate) struct ClerkFdw {
    rt: Runtime,
    token: Option<String>,
    client: Option<Client>,
    scan_result: Option<Vec<Row>>,
    tgt_cols: Vec<Column>,
}

impl ClerkFdw {
    const DEFAULT_BASE_URL: &'static str = "https://api.clerk.com/v1";

    // TODO: will have to incorportate offset at some point
    const PAGE_SIZE: usize = 500;

    fn build_url(&self, obj: &str, options: &HashMap<String, String>) -> String {
        match obj {
            "users" => {
                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let ret = format!("{}/users?limit={}", base_url, Self::PAGE_SIZE,);
                ret
            }
            "organizations" => {
                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let ret = format!("{}/organizations?limit={}", base_url, Self::PAGE_SIZE,);
                ret
            }
            "organization_memberships" => {
                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let org_id = options
                    .get("organization_id")
                    .expect("Organization ID required");
                let ret = format!(
                    "{}/organizations/{}/memberships?limit={}",
                    base_url,
                    org_id,
                    Self::PAGE_SIZE
                );
                ret
            }
            _ => {
                warning!("unsupported object: {:#?}", obj);
                return "".to_string();
            }
        }
    }
}

impl ForeignDataWrapper for ClerkFdw {
    fn new(options: &HashMap<String, String>) -> Self {
        let mut ret = Self {
            rt: create_async_runtime(),
            token: None,
            client: None,
            tgt_cols: Vec::new(),
            scan_result: None,
        };

        let token = if let Some(access_token) = options.get("api_key") {
            access_token.to_owned()
        } else {
            warning!("Cannot find api_key in options");
            let access_token = env::var("CLERK_API_KEY").unwrap();
            access_token
        };

        ret.token = Some(token);

        // create client
        let client = reqwest::Client::new();
        ret.client = Some(client);

        ret
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
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
        let api_key = self.token.as_ref().unwrap();

        if let Some(client) = &self.client {
            let mut result = Vec::new();

            if obj == "organization_memberships" {
                // Get all organizations first
                let org_url = self.build_url("organizations", options);

                self.rt.block_on(async {
                    let org_resp = client
                        .get(&org_url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .send()
                        .await;

                    if let Ok(org_res) = org_resp {
                        if org_res.status().is_success() {
                            let org_body = org_res.text().await.unwrap();
                            let org_json: JsonValue = serde_json::from_str(&org_body).unwrap();

                            if let Some(org_data) =
                                org_json.get("data").and_then(|data| data.as_array())
                            {
                                for org in org_data {
                                    if let Some(org_id) = org.get("id").and_then(|id| id.as_str()) {
                                        // Build the URL for memberships using org_id
                                        let membership_url = format!(
                                            "{}/organizations/{}/memberships?limit={}",
                                            Self::DEFAULT_BASE_URL,
                                            org_id,
                                            Self::PAGE_SIZE
                                        );

                                        let membership_resp = client
                                            .get(&membership_url)
                                            .header("Authorization", format!("Bearer {}", api_key))
                                            .send()
                                            .await;

                                        match membership_resp {
                                            Ok(mem_res) => {
                                                if mem_res.status().is_success() {
                                                    let mem_body = mem_res.text().await.unwrap();
                                                    let mem_json: JsonValue =
                                                        serde_json::from_str(&mem_body).unwrap();
                                                    // info!("mem_json: {:#?}", mem_json);

                                                    let mut rows = resp_to_rows(
                                                        &obj,
                                                        &mem_json,
                                                        &self.tgt_cols[..],
                                                    );
                                                    result.append(&mut rows);
                                                }
                                            }
                                            Err(_) => continue,
                                        };

                                        // Introduce a delay of 0.05 seconds
                                        std::thread::sleep(std::time::Duration::from_millis(50));
                                    }
                                }
                            }
                        }
                    }
                });
            } else {
                let url = self.build_url(&obj, options);

                // this is where i need to make changes
                self.rt.block_on(async {
                    let resp = client
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .send()
                        .await;

                    match resp {
                        Ok(res) => {
                            if res.status().is_success() {
                                let body = res.text().await.unwrap();
                                let json: JsonValue = serde_json::from_str(&body).unwrap();
                                let mut rows = resp_to_rows(&obj, &json, &self.tgt_cols[..]);
                                result.append(&mut rows);
                            } else {
                                warning!("Failed request with status: {}", res.status());
                            }
                        }
                        Err(error) => {
                            warning!("Error: {:#?}", error);
                            return;
                        }
                    };
                });
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
