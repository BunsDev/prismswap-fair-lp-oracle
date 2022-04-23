use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProxyQueryMsg {
    Price { asset_token: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct ProxyPriceResponse {
    pub rate: Decimal,     // rate denominated in base_denom
    pub last_updated: u64, // timestamp in seconds
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct TempProxyPriceResponse {
    pub rate: Decimal, // rate denominated in base_denom
    pub p1: Decimal,
    pub p2: Decimal,
    pub r1: Uint128,
    pub r2: Uint128,
    pub total_shares: Uint128,
    pub last_updated: u64, // timestamp in seconds
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ProxyBaseQuery {
    Base(ProxyQueryMsg),
}
