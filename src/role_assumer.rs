use rusoto_core::{DefaultCredentialsProvider, default_tls_client as GetRusotoTlsClient, ProvideAwsCredentials, Region};
use rusoto_sts::{StsClient, StsAssumeRoleSessionCredentialsProvider};
use shellexpand::tilde as TildeExpand;
use serde_json::{from_str as JsonFromStr, Map, Value as JsonValue};

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use errors::*;
use provide_shallow_credentials::ProvideShallowCredentials;

/// Handles the assuming of Roles. Also reads from places that contain mapping of
/// normalized names -> role to assume. (Like Rapture).
pub struct RoleAssumer {
  /// A Map of <Easy Role Name, Role>. To assume.
  roles_to_assume: HashMap<String, String>,
}

impl RoleAssumer {
  /// Creates a new thing that can assume roles.
  pub fn new() -> Result<Self> {
    let mut role_names = HashMap::new();

    let expanded_path = TildeExpand("~/.rapture/aliases.json").into_owned();
    let rapture_path = Path::new(&expanded_path);

    if rapture_path.exists() {
      let mut as_str = String::new();
      let mut file_handle = try!(File::open(rapture_path));
      try!(file_handle.read_to_string(&mut as_str));

      let potential_rapture_json = JsonFromStr(&as_str);
      if potential_rapture_json.is_ok() {
        let potential_rapture_json: JsonValue = potential_rapture_json.unwrap();
        let potential_rapture_json_obj: Option<&Map<String, JsonValue>> = potential_rapture_json.as_object();
        if potential_rapture_json_obj.is_some() {
          let rapture_obj = potential_rapture_json_obj.unwrap();
          for (key, value) in rapture_obj.into_iter() {
            if value.is_string() {
              role_names.insert(key.to_owned(), value.as_str().unwrap().to_owned());
            }
          }
        }
      }
    }

    Ok(Self { roles_to_assume: role_names })
  }

  pub fn assume_role(&self, to_assume: String) -> Result<ProvideShallowCredentials> {
    let tls_client = try!(GetRusotoTlsClient());
    let creds = try!(DefaultCredentialsProvider::new());
    let sts = StsClient::new(tls_client, creds, Region::UsEast1);

    let to_assume_frd = if self.roles_to_assume.contains_key(&to_assume) {
      self.roles_to_assume.get(&to_assume).unwrap().to_owned()
    } else {
      to_assume
    };

    Ok(ProvideShallowCredentials::new(try!(
      StsAssumeRoleSessionCredentialsProvider::new(
        sts,
        to_assume_frd,
        "ec2-rs-role-assumer".to_owned(),
        None,
        None,
        None,
        None,
      ).credentials()
    )))
  }

  pub fn get_default_creds(&self) -> Result<ProvideShallowCredentials> {
    Ok(ProvideShallowCredentials::new(
      try!(try!(DefaultCredentialsProvider::new()).credentials()),
    ))
  }
}
