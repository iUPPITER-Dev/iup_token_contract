use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Binary, StdError, StdResult,Uint128};
use cw20::{Cw20Coin, Expiration, Logo };
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
    pub logo_url_state: Option<String>,
}

#[cw_serde]
pub struct MemoInfo {
    action: String,
    player: String,
    cost: String,
}

#[cw_serde]
pub struct InitialBalance {
    pub address: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct MinterResponse {
    pub minter: String,
    pub cap: Option<Uint128>,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub marketing: Option<InstantiateMarketingInfo>,
    pub created_on_platform: Option<String>,
}

impl InstantiateMsg {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|m| m.cap)
    }

    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !self.has_valid_name() {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !self.has_valid_symbol() {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }

    fn has_valid_name(&self) -> bool {
        let bytes = self.name.as_bytes();
        if bytes.len() < 3 || bytes.len() > 50 {
            return false;
        }
        true
    }

    fn has_valid_symbol(&self) -> bool {
        let bytes = self.symbol.as_bytes();
        if bytes.len() < 3 || bytes.len() > 12 {
            return false;
        }
        for byte in bytes.iter() {
            if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
                return false;
            }
        }
        true
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    Transfer { recipient: String, amount: Uint128 },
    Burn { amount: Uint128 },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
       },
    IncreaseAllowance {
         spender: String,
         amount: Uint128,
         expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
       
    BurnFrom { owner: String, amount: Uint128 },
    Mint { recipient: String, amount: Uint128 },
    UpdateMinter { new_minter: Option<String> },
    UpdateMarketing {
            project: Option<String>,
            description: Option<String>,
            marketing: Option<String>,
    },
     
    UploadLogo(Logo),
    SetFeeGranter {
        address: Option<String>,
    },
    SetUpgradeAdmin {
        address: String,
    },
    UpdateConfig {
        new_config: Option<ConfigInfo>,
    },
   
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw20::TokenInfoResponse)]
    TokenInfo {},
    #[returns(cw20::BalanceResponse)]
    Balance { address: String },
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
    #[returns(cw20::AllAllowancesResponse)]
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::AllSpenderAllowancesResponse)]
    AllSpenderAllowances {
        spender: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(cw20::MarketingInfoResponse)]
    MarketingInfo {},
    #[returns(cw20::DownloadLogoResponse)]
    DownloadLogo {},
    // iUPPITER 추가 쿼리
    #[returns(FeeGranterResponse)]
    FeeGranter {},
    #[returns(Option<MinterResponse>)]
    Minter {},
    #[returns(TotalSupplyResponse)]
    TotalSupply {},
}

#[cw_serde]
pub struct ConfigInfo {
    pub is_upgrade_allowed: bool,
    pub upgrade_admin: Option<Addr>,
    pub marketing: Option<InstantiateMarketingInfo>,
    pub minter: Option<MinterResponse>,
    pub transfer_fee: Option<Uint128>,
    pub fee_collector: Option<Addr>,
    pub max_supply: Option<Uint128>,
}

#[cw_serde]
pub struct TotalSupplyResponse {
    pub total_supply: Uint128,
}

#[cw_serde]
pub struct FeeGranterResponse {
    pub fee_granter: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MigrateMsg {}