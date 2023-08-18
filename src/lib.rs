use clerk_rs::{clerk::Clerk, endpoints::ClerkGetEndpoint, ClerkConfiguration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use supabase_wrappers::{prelude::*, utils::get_vault_secret};
use tokio::runtime::Runtime;

// also display the role and organization

pgrx::pg_module_magic!();

#[wrappers_fdw(
    version = "0.1.0",
    author = "Jay Kothari",
    website = "https://tembo.io"
)]

pub struct ClerkFdw {
    row_cnt: i64,
    tgt_cols: Vec<Column>,
    users: Vec<User>,
    // organization: Option<String>,
}

fn fetch_users(api_key: &str) -> Vec<User> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let clerk_dev_api_token = api_key;
        let config =
            ClerkConfiguration::new(None, None, Some(clerk_dev_api_token.to_string()), None);
        let client = Clerk::new(config);
        let res = client.get(ClerkGetEndpoint::GetUserList).await.unwrap();
        serde_json::from_value(res).unwrap()
    })
}

impl ForeignDataWrapper for ClerkFdw {
    fn new(options: &HashMap<String, String>) -> Self {
        let users = match options.get("api_key") {
            Some(api_key) => Some(fetch_users(api_key)),
            None => require_option("api_key_id", options)
                .and_then(|key_id| get_vault_secret(&key_id))
                .map(|api_key| fetch_users(&api_key)),
        }
        .unwrap();
        // let organization = Some("tembo".to_string());
        Self {
            row_cnt: 0,
            tgt_cols: Vec::new(),
            users,
            // organization,
        }
    }

    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        _limit: &Option<Limit>,
        _options: &HashMap<String, String>,
    ) {
        self.row_cnt = 0;
        self.tgt_cols = columns.to_vec();
    }

    fn iter_scan(&mut self, row: &mut Row) -> Option<()> {
        if let Some(user) = self.users.get(self.row_cnt as usize) {
            for tgt_col in &self.tgt_cols {
                match tgt_col.name.as_str() {
                    "id" => row.push("id", Some(Cell::String(user.id.clone()))),
                    "first_name" => {
                        row.push("first_name", user.first_name.clone().map(Cell::String))
                    }
                    "last_name" => row.push("last_name", user.last_name.clone().map(Cell::String)),
                    "email" => row.push(
                        "email",
                        Some(Cell::String(
                            user.email_addresses
                                .first()
                                .map(|email| email.email_address.clone())
                                .unwrap_or_default(),
                        )),
                    ),
                    "gender" => row.push("gender", user.gender.clone().map(Cell::String)),
                    "created_at" => row.push("created_at", Some(Cell::I64(user.created_at as i64))),
                    "updated_at" => row.push("updated_at", Some(Cell::I64(user.updated_at as i64))),
                    "last_sign_in_at" => row.push(
                        "last_sign_in_at",
                        user.last_sign_in_at.map(|ts| Cell::I64(ts as i64)),
                    ),
                    "phone_numbers" => row.push(
                        "phone_numbers",
                        user.phone_numbers
                            .first()
                            .map(|phone| Cell::String(phone.clone())),
                    ),
                    "username" => row.push("username", user.username.clone().map(Cell::String)),
                    // "organization" => {
                    //     row.push("organization", self.organization.clone().map(Cell::String))
                    // }
                    _ => {}
                }
            }
            self.row_cnt += 1;
            return Some(());
        }
        None
    }

    fn end_scan(&mut self) {
        // Clean up resources here, if needed
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
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
    id: String, // figure out why it all is same
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
    family_name: String,
    given_name: String,
    google_id: String,
    id: String,
    label: Option<String>,
    object: String,
    picture: String,
    public_metadata: HashMap<String, String>,
    username: Option<String>,
    verification: Verification,
}

#[derive(Debug, Deserialize, Serialize)]
struct UnsafeMetadata {
    #[serde(default)]
    viewed_alpha_landing: bool,
}
