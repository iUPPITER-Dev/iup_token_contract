use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use cw20::{AllowanceResponse, Logo, MarketingInfoResponse};

use crate::msg::ConfigInfo;

#[cw_serde]
pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
    pub mint: Option<MinterData>,
}

#[cw_serde]
pub struct MinterData {
    pub minter: Addr,
    pub cap: Option<Uint128>,
}

#[cw_serde]
pub struct ExtendedTokenInfo {
    pub base: TokenInfo,
    pub admin: Addr,
    pub fee_granter: Option<Addr>,
    pub created_on_platform: Option<String>,
}

impl TokenInfo {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }
}

// 기본 CW20 상태 저장
pub const TOKEN_INFO: Item<TokenInfo> = Item::new("token_info");
pub const MARKETING_INFO: Item<MarketingInfoResponse> = Item::new("marketing_info");
pub const LOGO: Item<Logo> = Item::new("logo");
pub const BALANCES: Map<&Addr, Uint128> = Map::new("balance");
pub const ALLOWANCES: Map<(&Addr, &Addr), AllowanceResponse> = Map::new("allowance");
pub const ALLOWANCES_SPENDER: Map<(&Addr, &Addr), AllowanceResponse> = Map::new("allowance_spender");

// 확장 기능을 위한 추가 상태
pub const EXTENDED_INFO: Item<ExtendedTokenInfo> = Item::new("extended_info");
pub const CONFIG: Item<ConfigInfo> = Item::new("config");