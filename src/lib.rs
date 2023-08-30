use pgrx::{info, warning};
use pgrx::{pg_sys, prelude::*, JsonB};
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
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
    info!("code in body_to_rows");
    info!("obj_key: {}", obj_key);
    info!("normal_cols: {:#?}", normal_cols);
    info!("tgt_cols: {:#?}", tgt_cols);
    info!("resp[0]: {:#?}", resp);

    let mut result = Vec::new();

    info!("before match");
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
    info!("after match");

    for obj in objs {
        info!("obj: {:#?}", obj);
        let mut row = Row::new();

        // extract normal columns
        for tgt_col in tgt_cols {
            if let Some((src_name, col_name, col_type)) =
                normal_cols.iter().find(|(_, c, _)| c == &tgt_col.name)
            {
                let cell = obj
                    .as_object()
                    .and_then(|v| v.get(*src_name))
                    .and_then(|v| match *col_type {
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

        warning!("row: {:#?}", row);
        // put all properties into 'attrs' JSON column
        if tgt_cols.iter().any(|c| &c.name == "attrs") {
            let attrs = serde_json::from_str(&obj.to_string()).unwrap();
            row.push("attrs", Some(Cell::Json(JsonB(attrs))));
        }

        result.push(row);
    }
    info!("result: {:#?}", result);
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
                    ("email", "email", "string"),
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
        "junction_table" => {
            result = body_to_rows(
                resp,
                "junction_table",
                vec![
                    ("id", "id", "i64"),
                    ("user_id", "user_id", "string"),
                    ("organization_id", "organization_id", "string"),
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
    version = "0.2.0",
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
    const FDW_NAME: &str = "clerk_fdw";

    const DEFAULT_BASE_URL: &'static str = "https://api.clerk.com/v1/";

    // TODO: will have to incorportate offset at some point
    const PAGE_SIZE: usize = 500;

    // default maximum row count limit
    const DEFAULT_ROWS_LIMIT: usize = 10_000;

    fn build_url(&self, obj: &str, options: &HashMap<String, String>) -> String {
        match obj {
            "users" => {
                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let ret = format!("{}users?limit={}", base_url, Self::PAGE_SIZE,);
                ret
            }
            "organizations" => {
                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let ret = format!("{}organizations?limit={}", base_url, Self::PAGE_SIZE,);
                ret
            }
            "junction_table" => {
                warning!("junction_table is not supported");

                let base_url = Self::DEFAULT_BASE_URL.to_owned();
                let ret = format!("{}organizations?limit={}", base_url, Self::PAGE_SIZE,);
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

            let url = self.build_url(&obj, options);

            // this is where i need to make changes
            self.rt.block_on(async {
                let resp = client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .send()
                    .await;

                match resp {
                    Ok(mut res) => {
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

// Struct to hold user information
#[derive(Debug)]
struct UserInfo {
    id: String,
    first_name: Option<String>,
    last_name: Option<String>,
    email_addresses: String, // ideally it would be a Vec<String>
    gender: Option<String>,
    created_at: i64,
    updated_at: i64,
    last_sign_in_at: Option<i64>,
    phone_numbers: Vec<String>,
    username: Option<String>,
    organization_names: String, // ideally it would be a Vec<String>
    organization_roles: String, // ideally it would be a Vec<String>
                                // organizations_count: i64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Person {
    backup_code_enabled: bool,
    banned: bool,
    birthday: Option<String>,
    create_organization_enabled: bool,
    created_at: u64,
    delete_self_enabled: bool,
    email_addresses: Vec<EmailAddress>,
    external_accounts: Vec<ExternalAccount>,
    external_id: Option<String>,
    first_name: Option<String>,
    gender: Option<String>,
    has_image: bool,
    id: String,
    image_url: String,
    last_name: Option<String>,
    last_sign_in_at: Option<u64>,
    object: String,
    password_enabled: bool,
    phone_numbers: Vec<String>,
    primary_email_address_id: Option<String>,
    primary_phone_number_id: Option<String>,
    primary_web3_wallet_id: Option<String>,
    private_metadata: HashMap<String, String>,
    profile_image_url: String,
    public_metadata: HashMap<String, String>,
    saml_accounts: Vec<String>,
    totp_enabled: bool,
    two_factor_enabled: bool,
    unsafe_metadata: UnsafeMetadata,
    updated_at: u64,
    username: Option<String>,
    web3_wallets: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct EmailAddress {
    email_address: String,
    id: String,
    linked_to: Vec<LinkedTo>,
    object: String,
    reserved: bool,
    verification: Verification,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinkedTo {
    id: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Verification {
    attempts: Option<i32>,
    expire_at: Option<u64>,
    status: String,
    strategy: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ExternalAccount {
    approved_scopes: String,
    email_address: String,
    family_name: Option<String>,
    given_name: Option<String>,
    google_id: Option<String>,
    id: String,
    label: Option<String>,
    object: String,
    picture: Option<String>,
    public_metadata: HashMap<String, String>,
    username: Option<String>,
    verification: Verification,
}

#[derive(Debug, Deserialize, Serialize)]
struct UnsafeMetadata {
    #[serde(default)]
    viewed_alpha_landing: bool,
}
