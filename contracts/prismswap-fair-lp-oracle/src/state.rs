use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

use crate::msg::ConfigResponse;

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub prism_oracle_addr: Addr,
}

impl Config {
    pub fn as_res(&self) -> ConfigResponse {
        ConfigResponse {
            prism_oracle_addr: self.prism_oracle_addr.to_string(),
        }
    }
}
