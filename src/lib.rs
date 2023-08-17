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
    author = "Your Name",
    website = "https://yourwebsite.com"
)]

pub struct ClerkFdw {
    row_cnt: i64,
    tgt_cols: Vec<Column>,
    users: Vec<User>,
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
        Self {
            row_cnt: 0,
            tgt_cols: Vec::new(),
            users,
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
                    "name" => row.push(
                        "name",
                        Some(Cell::String(format!(
                            "{} {}",
                            user.first_name.clone().unwrap_or_default(),
                            user.last_name.clone().unwrap_or_default()
                        ))),
                    ),
                    "email" => row.push(
                        "email",
                        Some(Cell::String(
                            user.email_addresses
                                .first()
                                .map(|email| email.email_address.clone())
                                .unwrap_or_default(),
                        )),
                    ),
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
    created_at: u64, // use this
    delete_self_enabled: bool,
    email_addresses: Vec<EmailAddress>,
    external_accounts: Vec<ExternalAccount>,
    external_id: Option<String>,
    first_name: Option<String>, // split out first sanme and last name
    gender: Option<String>,     // use this
    has_image: bool,
    id: String, // figure out why it all is same
    image_url: String,
    last_name: Option<String>,
    last_sign_in_at: Option<u64>, // use this
    object: String,
    password_enabled: bool,
    phone_numbers: Vec<String>, // use this
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
    updated_at: u64,          // use this
    username: Option<String>, // use this
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
