#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, StdError,
    StdResult, Uint128, WasmQuery,
};
use terra_cosmwasm::{ExchangeRatesResponse, TerraQuerier};

use cw2::set_contract_version;

use cw20::Cw20QueryMsg::Minter;
use cw20::MinterResponse;

use prismswap::asset::AssetInfo;
use prismswap::pair::{PoolResponse as PrismswapPoolResponse, QueryMsg as PrismswapQueryMsg};

use tefi_oracle::hub::{HubQueryMsg, PriceResponse as HubPriceResponse};
use tefi_oracle::proxy::{ProxyPriceResponse, ProxyQueryMsg};

use crate::msg::{ConfigResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use crate::ContractError;
/// This module is purely a workaround that lets us ignore lints for all the code the `construct_uint!`
/// macro generates
#[allow(clippy::all)]
mod uints {
    uint::construct_uint! {
        pub struct U256(4);
    }
}

/// Used internally - we don't want to leak this type since we might change the implementation in the future
use uints::U256;

// version info for migration info
const CONTRACT_NAME: &str = "prismswap-fair-lp-oracle";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        prism_oracle_addr: deps.api.addr_validate(&msg.prism_oracle_addr)?,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let res = match msg {
        // Any custom query msgs
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        // Implementation of the queries required by proxy contract standard
        QueryMsg::Base(proxy_msg) => match proxy_msg {
            ProxyQueryMsg::Price { asset_token } => {
                to_binary(&query_price(deps, env, asset_token)?)
            }
        },
    };

    res.map_err(|err| err.into())
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;

    Ok(config.as_res())
}

pub fn query_price(deps: Deps, env: Env, asset_token: String) -> StdResult<ProxyPriceResponse> {
    // minter of LP token is pair contract
    let minter_response: MinterResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: asset_token.to_string(),
            msg: to_binary(&Minter {})?,
        }))?;

    let pool_response: PrismswapPoolResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: minter_response.minter,
            msg: to_binary(&PrismswapQueryMsg::Pool {})?,
        }))?;

    let primary_asset_info = &pool_response.assets[0].info;
    let secondary_asset_info = &pool_response.assets[1].info;

    let primary_depth = pool_response
        .assets
        .iter()
        .find(|asset| asset.info == *primary_asset_info)
        .ok_or_else(|| StdError::generic_err("cannot find primary asset in pool response"))?
        .amount;
    let secondary_depth = pool_response
        .assets
        .iter()
        .find(|asset| asset.info == *secondary_asset_info)
        .ok_or_else(|| StdError::generic_err("cannot find secondary asset in pool response"))?
        .amount;
    let total_shares = pool_response.total_share;

    let primary_price = query_external_price(&deps, &env, &primary_asset_info)?;
    let secondary_price = query_external_price(&deps, &env, &secondary_asset_info)?;

    let primary_value = U256::from(u128::from(primary_depth * primary_price));
    let secondary_value = U256::from(u128::from(secondary_depth * secondary_price));
    let pool_value = U256::from(2) * (primary_value * secondary_value).integer_sqrt();

    let pool_value_u128 = Uint128::new(pool_value.as_u128());
    let lp_token_value = Decimal::from_ratio(pool_value_u128, total_shares);

    Ok(ProxyPriceResponse {
        rate: lp_token_value,
        last_updated: env.block.time.seconds(),
    })
}

fn query_external_price(deps: &Deps, _env: &Env, asset: &AssetInfo) -> StdResult<Decimal> {
    match asset {
        AssetInfo::Native(denom) => {
            if denom == "uusd" {
                Ok(Decimal::one())
            } else {
                let native_price = query_native_price(deps, denom.clone())?;
                Ok(native_price.0)
            }
        }
        AssetInfo::Cw20(contract_addr) => {
            let config = CONFIG.load(deps.storage)?;
            let hub_price_response: HubPriceResponse =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: config.prism_oracle_addr.to_string(),
                    msg: to_binary(&HubQueryMsg::Price {
                        asset_token: contract_addr.to_string(),
                        timeframe: None,
                    })?,
                }))?;
            Ok(hub_price_response.rate)
        }
    }
}

fn query_native_price(deps: &Deps, denom: String) -> StdResult<(Decimal, u64)> {
    let terra_querier = TerraQuerier::new(&deps.querier);
    // Get the price of a native asset in uusd (on-chain)
    let res: ExchangeRatesResponse =
        terra_querier.query_exchange_rates(denom, vec!["uusd".to_string()])?;

    Ok((res.exchange_rates[0].exchange_rate, u64::MAX))
}
