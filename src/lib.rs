use clerk_rs::{
    apis::organization_memberships_api::OragnizationMebership, apis::users_api::User, clerk::Clerk,
    models::organization_membership::Role, ClerkConfiguration,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use supabase_wrappers::{prelude::*, utils::get_vault_secret};
use tokio::runtime::Runtime;

pgrx::pg_module_magic!();

// Foreign Data Wrapper (FDW) attributes
#[wrappers_fdw(
    version = "0.1.2",
    author = "Jay Kothari",
    website = "https://tembo.io"
)]

// TODO: users should be Option type
// not sure if having API key in the self object is a good idea
pub struct ClerkFdw {
    row_cnt: i64,
    tgt_cols: Vec<Column>,
    clerk_client: Option<Clerk>,
    api_key: Option<String>,
    users: Vec<UserInfo>, // User Information
}

fn create_clerk_client(api_key: &str) -> Clerk {
    let config = ClerkConfiguration::new(None, None, Some(api_key.to_string()), None);
    Clerk::new(config)
}

async fn get_users_reqwest(url: &str, api_key: &str) -> Result<Vec<Person>, reqwest::Error> {
    // Set up the request client
    let client = reqwest::Client::new();

    // Making the GET request
    let res_result = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key.to_string()))
        .send()
        .await;

    let res = match res_result {
        Ok(file) => file,
        Err(error) => {
            eprintln!("Error: {:#?}", error);
            return Err(error);
        }
    };

    let users_json: Value = res.json().await?;

    // Convert response to a JSON object (assuming the response is a JSON)
    // not able to handle this unwrap because of return type
    let users: Vec<Person> = serde_json::from_value(users_json)
        .map_err(|err| {
            eprintln!("{err}");
            err
        })
        .unwrap();

    Ok(users)
}

// Function to fetch users from the Clerk API
// Need to properly handle the errors instead of empty return statements
fn fetch_users(clerk_client: &Option<Clerk>, api_key: &str) -> Vec<UserInfo> {
    let rt = Runtime::new().unwrap();
    let mut user_info_list: Vec<UserInfo> = Vec::new();
    let clerk = match clerk_client {
        Some(client) => client,
        None => {
            eprintln!("Error: No Clerk client provided");
            return Vec::new();
        }
    };

    rt.block_on(async {
        // Initialize the Clerk client
        let clerk_dev_api_token = api_key;

        let json_data_result = get_users_reqwest(
            "https://api.clerk.com/v1/users?limit=500",
            &clerk_dev_api_token,
        )
        .await;

        let json_data = match json_data_result {
            Ok(data) => data,
            Err(error) => {
                eprintln!("Error: {:#?}", error);
                return;
            }
        };

        // Iterate through the users and fetch their organization memberships
        for user in json_data {
            // Fetch the organization memberships of the user
            let org_data_result =
                User::users_get_organization_memberships(&clerk, &user.id, Some(0.0), Some(0.0))
                    .await;

            let org_data = match org_data_result {
                Ok(data) => data,
                Err(error) => {
                    eprintln!("Error: {:#?}", error);
                    return;
                }
            };
            let mut organization_names = Vec::new();
            let mut organization_roles = Vec::new();
            for membership in org_data.data {
                if let Some(organization) = membership.organization {
                    organization_names.push(organization.name);
                    // get data about the roles of the user in the organization
                    let org_membership_list_result =
                        OragnizationMebership::list_organization_memberships(
                            &clerk,
                            &organization.id,
                            Some(0.0),
                            Some(0.0),
                        )
                        .await;

                    let org_membership_list = match org_membership_list_result {
                        Ok(data) => data,
                        Err(error) => {
                            eprintln!("Error: {:#?}", error);
                            return;
                        }
                    };

                    // Iterate through the organization memberships to obtain the role
                    for org_mem in org_membership_list.data {
                        // Check if the user ID in org_mem matches the current user's ID
                        if let Some(public_user_data) = org_mem.public_user_data {
                            if let Some(org_user_id) = public_user_data.user_id {
                                if org_user_id == user.id {
                                    if let Some(role) = org_mem.role {
                                        // Push the respective role to the organization_roles vector
                                        organization_roles.push(role_to_string(&role));
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Create a UserInfo struct and push it to the user_info_list
            let user_info = UserInfo {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
                email_addresses: user
                    .email_addresses
                    .first()
                    .map(|email| email.email_address.clone())
                    .unwrap_or_default(),
                gender: user.gender,
                created_at: user.created_at as i64,
                updated_at: user.updated_at as i64,
                last_sign_in_at: user.last_sign_in_at.map(|ts| ts as i64),
                phone_numbers: user.phone_numbers,
                username: user.username,
                organization_names: organization_names.join(","),
                organization_roles: organization_roles.join(","),
            };
            user_info_list.push(user_info);
        }
    });
    return user_info_list;
}

// Function to convert role enum to string
fn role_to_string(role: &Role) -> String {
    match role {
        Role::Admin => "Admin".to_string(),
        Role::BasicMember => "BasicMember".to_string(),
    }
}

impl ForeignDataWrapper for ClerkFdw {
    // Constructor for the FDW
    fn new(options: &HashMap<String, String>) -> Self {
        let users = Vec::new();

        let api_key = options.get("api_key").cloned();

        let clerk_client = None;

        Self {
            row_cnt: 0,
            tgt_cols: Vec::new(),
            clerk_client,
            api_key,
            users,
        }
    }

    // Begin the scan operation
    fn begin_scan(
        &mut self,
        _quals: &[Qual],
        columns: &[Column],
        _sorts: &[Sort],
        _limit: &Option<Limit>,
        _options: &HashMap<String, String>,
    ) {
        if let Some(ref api_key) = self.api_key {
            let clerk_client = create_clerk_client(api_key);
            self.clerk_client = Some(clerk_client);
            self.users = fetch_users(&self.clerk_client, api_key);
        } else {
            // Handle the case where API key is not available
            eprintln!("Error: No API key available");
        }

        self.row_cnt = 0;
        self.tgt_cols = columns.to_vec();
    }

    // Iterate through the scan results, populating rows with user data
    // TODO: Convert type of created_at, updated_at, last_sign_in_at to timestamp
    // TODO: Convert type of phone_numbers, emails, organizations and roles to array
    fn iter_scan(&mut self, row: &mut Row) -> Option<()> {
        if let Some(user) = self.users.get(self.row_cnt as usize) {
            for tgt_col in &self.tgt_cols {
                match tgt_col.name.as_str() {
                    "id" => row.push("id", Some(Cell::String(user.id.clone()))),
                    "first_name" => {
                        row.push("first_name", user.first_name.clone().map(Cell::String))
                    }
                    "last_name" => row.push("last_name", user.last_name.clone().map(Cell::String)),
                    "email" => row.push("email", Some(Cell::String(user.email_addresses.clone()))),
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
                    "organization" => row.push(
                        "organization",
                        Some(Cell::String(user.organization_names.clone())),
                    ),
                    "role" => row.push("role", Some(Cell::String(user.organization_roles.clone()))),
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

// Struct to hold user information
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
