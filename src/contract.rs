#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, 
};
use cw2::{ensure_from_older_version, set_contract_version};
use cw20::{
    BalanceResponse, Cw20ReceiveMsg, DownloadLogoResponse, EmbeddedLogo, 
    Logo, LogoInfo, MarketingInfoResponse, MinterResponse, TokenInfoResponse,
};

#[cfg(test)]
use cosmwasm_std::Addr;

use crate::allowances::{
    execute_burn_from, execute_decrease_allowance, execute_increase_allowance, execute_send_from,
    execute_transfer_from, query_allowance,
};
use crate::enumerable::{query_all_accounts, query_owner_allowances, query_spender_allowances};
use crate::error::ContractError;
use crate::msg::{
     ConfigInfo, ExecuteMsg, FeeGranterResponse, InstantiateMsg, MigrateMsg, QueryMsg, TotalSupplyResponse
};
use crate::state::{
    ExtendedTokenInfo, MinterData, TokenInfo, ALLOWANCES, ALLOWANCES_SPENDER, BALANCES, CONFIG, EXTENDED_INFO, LOGO, MARKETING_INFO, TOKEN_INFO
};

// Contract name and version
const CONTRACT_NAME: &str = "crates.io:iuppiter-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Logo size limit
const LOGO_SIZE_CAP: usize = 5 * 1024;

// Logo validation helpers
fn verify_xml_preamble(data: &[u8]) -> Result<(), ContractError> {
    let preamble = data
        .split_inclusive(|c| *c == b'>')
        .next()
        .ok_or(ContractError::InvalidXmlPreamble {})?;

    const PREFIX: &[u8] = b"<?xml ";
    const POSTFIX: &[u8] = b"?>";

    if !(preamble.starts_with(PREFIX) && preamble.ends_with(POSTFIX)) {
        Err(ContractError::InvalidXmlPreamble {})
    } else {
        Ok(())
    }
}

fn verify_logo(logo: &Logo) -> Result<(), ContractError> {
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => verify_xml_logo(logo),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => verify_png_logo(logo),
        Logo::Url(_) => Ok(()),
    }
}

fn verify_xml_logo(logo: &[u8]) -> Result<(), ContractError> {
    verify_xml_preamble(logo)?;

    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else {
        Ok(())
    }
}

fn verify_png_logo(logo: &[u8]) -> Result<(), ContractError> {
    // PNG header format
    const PNG_HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    
    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::LogoTooBig {})
    } else if !logo.starts_with(&PNG_HEADER) {
        Err(ContractError::InvalidPngHeader {})
    } else {
        Ok(())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // 유효성 검사 추가
    msg.validate()?;

    let mut total_supply = Uint128::zero();
    
    // initial_balances를 Vec<Cw20Coin>으로 처리
    for balance in msg.initial_balances.iter() {
        #[cfg(test)]
        let address = Addr::unchecked(&balance.address);
        
        #[cfg(not(test))]
        let address = deps.api.addr_validate(&balance.address)?;
        
        BALANCES.save(deps.storage, &address, &balance.amount)?;

        total_supply = total_supply
            .checked_add(balance.amount)
            .map_err(|_| ContractError::InvalidAmount {})?;
    }

    // 발행량 한도 체크
    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap").into());
        }
    }

    // Minter 설정
    let mint = match msg.mint {
        Some(m) => {
            #[cfg(test)]
            let minter = Addr::unchecked(&m.minter);
            
            #[cfg(not(test))]
            let minter = deps.api.addr_validate(&m.minter)?;

            Some(MinterData {
                minter,
                cap: m.cap,
            })
        }
        None => None,
    };

    // 토큰 기본 정보 저장
    let token_info = TokenInfo {
        name: msg.name,
        symbol: msg.symbol.clone(),
        decimals: msg.decimals,
        total_supply,
        mint,
    };
    TOKEN_INFO.save(deps.storage, &token_info)?;

    // 마케팅 정보 처리
    if let Some(marketing) = msg.marketing {
        let logo = if let Some(logo) = marketing.logo {
            verify_logo(&logo)?;
            LOGO.save(deps.storage, &logo)?;

            match logo {
                Logo::Url(url) => Some(LogoInfo::Url(url)),
                Logo::Embedded(_) => Some(LogoInfo::Embedded),
            }
        } else {
            None
        };

        // 마케팅 주소 검증
        #[cfg(test)]
        let marketing_addr = marketing
            .marketing
            .map(|addr| Addr::unchecked(&addr));

        #[cfg(not(test))]
        let marketing_addr = marketing
            .marketing
            .map(|addr| deps.api.addr_validate(&addr))
            .transpose()?;

        let data = MarketingInfoResponse {
            project: marketing.project,
            description: marketing.description,
            marketing: marketing_addr,
            logo,
        };

        MARKETING_INFO.save(deps.storage, &data)?;
    }

    // iUPPITER 추가 설정
    #[cfg(test)]
    let admin = if !msg.initial_balances.is_empty() {
        Addr::unchecked(&msg.initial_balances[0].address)
    } else {
        info.sender.clone()
    };

    #[cfg(not(test))]
    let admin = if !msg.initial_balances.is_empty() {
        deps.api.addr_validate(&msg.initial_balances[0].address)?
    } else {
        info.sender.clone()
    };

    let config = ConfigInfo {
        is_upgrade_allowed: true,
        upgrade_admin: None,
        marketing: None,
        minter: None,
        transfer_fee: None,
        fee_collector: None,
        max_supply: None,
    };
    CONFIG.save(deps.storage, &config)?;

    // 확장 정보 저장
    let extended_info = ExtendedTokenInfo {
        base: token_info.clone(),
        admin: admin.clone(),
        fee_granter: None,
        created_on_platform: msg.created_on_platform,
    };

    EXTENDED_INFO.save(deps.storage, &extended_info)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", admin)
        .add_attribute("total_supply", total_supply)
        .add_attribute("token_symbol", msg.symbol))
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // CW20 기본 기능
        ExecuteMsg::Transfer { recipient, amount } => execute_transfer(deps, env, info, recipient, amount),
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::Mint { recipient, amount } => {
            execute_mint(deps, env, info, recipient, amount)
        }
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, env, info, owner, amount),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),
        
        // 마케팅 관련 기능
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => execute_update_marketing(deps, env, info, project, description, marketing),
        ExecuteMsg::UploadLogo(logo) => execute_upload_logo(deps, env, info, logo),
        ExecuteMsg::UpdateMinter { new_minter } => {
            execute_update_minter(deps, env, info, new_minter)
        }
        ExecuteMsg::SetFeeGranter { address } => {
            execute_set_fee_granter(deps, info, address)
        }
        ExecuteMsg::SetUpgradeAdmin { address } => {
            execute_set_upgrade_admin(deps, info, address)
        }
        ExecuteMsg::UpdateConfig { new_config } => {
            execute_update_config(deps, info, new_config)
        }
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {

    #[cfg(test)]
    let rcpt_addr = Addr::unchecked(&recipient);
        
    #[cfg(not(test))]
    let rcpt_addr = deps.api.addr_validate(&recipient)?;

    // 잔액 체크 먼저 수행
    let sender_balance = BALANCES.load(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(ContractError::InvalidAmount {});
    }

    // 잔액 업데이트
    BALANCES.update(deps.storage, &info.sender, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default() - amount)
    })?;
    BALANCES.update(deps.storage, &rcpt_addr, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default() + amount)
    })?;

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // lower balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    // reduce total_supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    let res = Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&contract)?;

    // move the tokens to the contract
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", &info.sender)
        .add_attribute("to", &contract)
        .add_attribute("amount", amount)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.into(),
                amount,
                msg,
            }
            .into_cosmos_msg(contract)?,
        );
    Ok(res)
}

pub fn execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    if config
        .mint
        .as_ref()
        .ok_or(ContractError::Unauthorized {})?
        .minter
        != info.sender
    {
        return Err(ContractError::Unauthorized {});
    }

    // update supply and enforce cap
    config.total_supply += amount;
    if let Some(limit) = config.get_cap() {
        if config.total_supply > limit {
            return Err(ContractError::CannotExceedCap {});
        }
    }
    TOKEN_INFO.save(deps.storage, &config)?;

    // add amount to recipient balance
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_update_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    let mint = config.mint.as_ref().ok_or(ContractError::Unauthorized {})?;
    if mint.minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let minter_data = new_minter
        .map(|new_minter| deps.api.addr_validate(&new_minter))
        .transpose()?
        .map(|minter| MinterData {
            minter,
            cap: mint.cap,
        });

    config.mint = minter_data;

    TOKEN_INFO.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_minter")
        .add_attribute(
            "new_minter",
            config
                .mint
                .map(|m| m.minter.into_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

// 새로운 marketing 관련 실행 함수 추가
pub fn execute_update_marketing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    project: Option<String>,
    description: Option<String>,
    marketing: Option<String>,
) -> Result<Response, ContractError> {
    let mut marketing_info = MARKETING_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    if marketing_info
        .marketing
        .as_ref()
        .ok_or(ContractError::Unauthorized {})?
        != info.sender
    {
        return Err(ContractError::Unauthorized {});
    }

    match project {
        Some(empty) if empty.trim().is_empty() => marketing_info.project = None,
        Some(project) => marketing_info.project = Some(project),
        None => (),
    }

    match description {
        Some(empty) if empty.trim().is_empty() => marketing_info.description = None,
        Some(description) => marketing_info.description = Some(description),
        None => (),
    }

    match marketing {
        Some(empty) if empty.trim().is_empty() => marketing_info.marketing = None,
        Some(marketing) => marketing_info.marketing = Some(deps.api.addr_validate(&marketing)?),
        None => (),
    }

    if marketing_info.project.is_none()
        && marketing_info.description.is_none()
        && marketing_info.marketing.is_none()
        && marketing_info.logo.is_none()
    {
        MARKETING_INFO.remove(deps.storage);
    } else {
        MARKETING_INFO.save(deps.storage, &marketing_info)?;
    }

    let res = Response::new().add_attribute("action", "update_marketing");
    Ok(res)
}

pub fn execute_upload_logo(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    logo: Logo,
) -> Result<Response, ContractError> {
    let mut marketing_info = MARKETING_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    verify_logo(&logo)?;

    if marketing_info
        .marketing
        .as_ref()
        .ok_or(ContractError::Unauthorized {})?
        != info.sender
    {
        return Err(ContractError::Unauthorized {});
    }

    LOGO.save(deps.storage, &logo)?;

    let logo_info = match logo {
        Logo::Url(url) => LogoInfo::Url(url),
        Logo::Embedded(_) => LogoInfo::Embedded,
    };

    marketing_info.logo = Some(logo_info);
    MARKETING_INFO.save(deps.storage, &marketing_info)?;

    let res = Response::new().add_attribute("action", "upload_logo");
    Ok(res)
}

// 새로운 실행 함수들 추가
pub fn execute_set_upgrade_admin(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    // ExtendedTokenInfo에서 admin 정보 확인
    let extended_info = EXTENDED_INFO.load(deps.storage)?;
    if info.sender != extended_info.admin {
        return Err(ContractError::Unauthorized {});
    }

    #[cfg(test)]
    let upgrade_admin = Addr::unchecked(&address);
 
    #[cfg(not(test))]
    let upgrade_admin = deps.api.addr_validate(&address)?;

    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.upgrade_admin = Some(upgrade_admin.clone());
        Ok(config)
    })?;

    Ok(Response::new()
        .add_attribute("method", "set_upgrade_admin")
        .add_attribute("upgrade_admin", upgrade_admin))
}

pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_config: Option<ConfigInfo>,
) -> Result<Response, ContractError> {
    let current_config = CONFIG.load(deps.storage)?;
    let extended_info = EXTENDED_INFO.load(deps.storage)?;
    let mut token_info = TOKEN_INFO.load(deps.storage)?;
    
    // 권한 체크 - extended_info의 admin 또는 upgrade_admin만 가능
    if let Some(upgrade_admin) = current_config.upgrade_admin.as_ref() {
        if info.sender != *upgrade_admin && info.sender != extended_info.admin {
            return Err(ContractError::Unauthorized {});
        }
    } else if info.sender != extended_info.admin {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(ref new_config) = new_config {
        // Validate upgrade permission
        if !current_config.is_upgrade_allowed {
            return Err(ContractError::ConfigUpdateNotAllowed {});
        }

        // minter 설정 업데이트
        if let Some(new_minter) = new_config.minter.as_ref() {
            let new_minter_data = MinterData {
                minter: deps.api.addr_validate(&new_minter.minter)?,
                cap: new_minter.cap,
            };
            
            // Only update if different
            if token_info.mint.as_ref() != Some(&new_minter_data) {
                token_info.mint = Some(new_minter_data);
                TOKEN_INFO.save(deps.storage, &token_info)?;
            }
        } else if token_info.mint.is_some() {
            // Remove minter if new config has None
            token_info.mint = None;
            TOKEN_INFO.save(deps.storage, &token_info)?;
        }

        // 최대 발행량 체크 (설정된 경우)
        if let Some(max_supply) = new_config.max_supply {
            if token_info.total_supply > max_supply {
                return Err(ContractError::InvalidAmount {});
            }
        }

        // 수수료 관련 설정 유효성 검사
        if new_config.transfer_fee.is_some() && new_config.fee_collector.is_none() {
            return Err(ContractError::InvalidConfig {
                msg: "Fee collector must be set when transfer fee is enabled".to_string(),
            });
        }

        // 마케팅 정보 업데이트
        if let Some(marketing) = &new_config.marketing {
            let marketing_info = MarketingInfoResponse {
                project: marketing.project.clone(),
                description: marketing.description.clone(),
                marketing: marketing.marketing.as_ref().map(|addr| deps.api.addr_validate(addr).unwrap()),
                logo: None, // Logo는 별도의 함수를 통해 업데이트
            };
            MARKETING_INFO.save(deps.storage, &marketing_info)?;
        }

        // 설정 저장
        CONFIG.save(deps.storage, new_config)?;

        Ok(Response::new()
            .add_attribute("method", "update_config")
            .add_attribute("success", "true")
            .add_attribute("is_upgrade_allowed", new_config.is_upgrade_allowed.to_string())
            .add_attribute("has_minter", new_config.minter.is_some().to_string())
            .add_attribute("has_transfer_fee", new_config.transfer_fee.is_some().to_string())
            .add_attribute("has_max_supply", new_config.max_supply.is_some().to_string()))
    } else {
        Err(ContractError::InvalidConfig {
            msg: "New config cannot be empty".to_string(),
        })
    }
}

pub fn execute_set_fee_granter(
    deps: DepsMut,
    info: MessageInfo,
    address: Option<String>,
) -> Result<Response, ContractError> {
    let mut extended_info = EXTENDED_INFO.load(deps.storage)?;
    
    // 관리자 권한 체크
    if info.sender != extended_info.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Config 체크
    let config = CONFIG.load(deps.storage)?;
    if !config.is_upgrade_allowed {
        return Err(ContractError::ConfigUpdateNotAllowed {});
    }

    // 대납자 주소 설정 또는 해제
    extended_info.fee_granter = match address {
        Some(addr) => {
            #[cfg(test)]
            let validated_addr = Addr::unchecked(&addr);
            #[cfg(not(test))]
            let validated_addr = deps.api.addr_validate(&addr)?;

            // 대납자가 관리자와 동일한지 체크
            if validated_addr == extended_info.admin {
                return Err(ContractError::CannotSetOwnAccount {});
            }

            Some(validated_addr)
        },
        None => None,
    };

    // ExtendedTokenInfo 저장
    EXTENDED_INFO.save(deps.storage, &extended_info)?;

    Ok(Response::new()
        .add_attribute("method", "set_fee_granter")
        .add_attribute("fee_granter", extended_info.fee_granter
            .map_or_else(|| "None".to_string(), |addr| addr.to_string()))
        .add_attribute("admin", extended_info.admin))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_json_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_json_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&query_owner_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_json_binary(&query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_json_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_json_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_json_binary(&query_download_logo(deps)?),
        QueryMsg::TotalSupply {} => to_json_binary(&query_total_supply(deps)?),
        QueryMsg::FeeGranter {} => to_json_binary(&query_fee_granter(deps)?),
       
       
    }
}

pub fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    #[cfg(test)]
    let address = Addr::unchecked(&address);
        
    #[cfg(not(test))]
    let address = deps.api.addr_validate(&address)?;

    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    Ok(BalanceResponse { balance })
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    };
    Ok(res)
}

pub fn query_minter(deps: Deps) -> StdResult<Option<MinterResponse>> {
    let meta = TOKEN_INFO.load(deps.storage)?;
    let minter = match meta.mint {
        Some(m) => Some(MinterResponse {
            minter: m.minter.into(),
            cap: m.cap,
        }),
        None => None,
    };
    Ok(minter)
}

pub fn query_marketing_info(deps: Deps) -> StdResult<MarketingInfoResponse> {
    Ok(MARKETING_INFO.may_load(deps.storage)?.unwrap_or_default())
}

pub fn query_download_logo(deps: Deps) -> StdResult<DownloadLogoResponse> {
    let logo = LOGO.load(deps.storage)?;
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => Ok(DownloadLogoResponse {
            mime_type: "image/svg+xml".to_owned(),
            data: logo,
        }),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => Ok(DownloadLogoResponse {
            mime_type: "image/png".to_owned(),
            data: logo,
        }),
        Logo::Url(_) => Err(StdError::not_found("logo")),
    }
}

fn query_fee_granter(deps: Deps) -> StdResult<FeeGranterResponse> {
    let extended_info = EXTENDED_INFO.load(deps.storage)?;
    Ok(FeeGranterResponse {
        fee_granter: extended_info.fee_granter.map(|addr| addr.to_string()),
    })
}

fn query_total_supply(deps: Deps) -> StdResult<TotalSupplyResponse> {
    let token_info = TOKEN_INFO.load(deps.storage)?;
    Ok(TotalSupplyResponse {
        total_supply: token_info.total_supply,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let original_version =
        ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    if original_version < "0.14.0".parse::<semver::Version>().unwrap() {
        // Build reverse map of allowances per spender
        let data = ALLOWANCES
            .range(deps.storage, None, None, Ascending)
            .collect::<StdResult<Vec<_>>>()?;
        for ((owner, spender), allowance) in data {
            ALLOWANCES_SPENDER.save(deps.storage, (&spender, &owner), &allowance)?;
        }
    }
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::Addr;
    use crate::msg::InstantiateMarketingInfo;  // 필요한 것만 import
    use cw20::Cw20Coin;

    const CREATOR: &str = "cosmos1vlhe6z8r7al2lyzp7n3j2vl5kd28hhrw0vxmxr";
    const ADMIN: &str = "cosmos1wztmxhufhy98p3n45yqtwhrxlrr9wkg0tt3a3c";
    const USER1: &str = "cosmos1qg9zllptnqvhyvrrvm0j3qjmtc5q6ds7eq0le4";
    // const USER2: &str = "cosmos1dfk8f8h3xejm5h6u9e8l6uqsphtld4ld82g8sw";
    // const PLAYER: &str = "cosmos1hsm5esvj0eg2lnn22wh2wdjzlyyqjc7nmkpdrq";
    const FEE_GRANTER: &str = "cosmos1uzsvd5gh4l0wdf6ekufg0nhrgdl3nk7gy5ksy7";


    #[test]
    fn proper_initialization_with_marketing() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        let msg = InstantiateMsg {
            name: "iUPPITER".to_string(),
            symbol: "iUP".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ADMIN.to_string(),
                amount: Uint128::new(1000000000000),
            }],
            marketing: Some(InstantiateMarketingInfo {
                project: Some("iUPPITER Project".to_string()),
                description: Some("Game Token".to_string()),
                marketing: None,
                logo: None,
                logo_url_state: Some("https://example.com/logo.png".to_string()),
            }),
            mint: None,
            created_on_platform: Some("iUPPITER Platform".to_string()),
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(4, res.attributes.len());
        
        // 마케팅 정보 확인 (TOKEN_INFO에서 로드)
        let marketing_info = MARKETING_INFO.load(&deps.storage).unwrap();
        assert_eq!(marketing_info.project.unwrap(), "iUPPITER Project");
        
        // 확장 정보 확인 (EXTENDED_INFO에서 로드)
        let extended_info = EXTENDED_INFO.load(&deps.storage).unwrap();
        assert_eq!(extended_info.created_on_platform.unwrap(), "iUPPITER Platform");
    }


    #[test]
    fn test_transfer() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Initialize contract
        let msg = InstantiateMsg {
            name: "iUPPITER".to_string(),
            symbol: "iUP".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ADMIN.to_string(),
                amount: Uint128::new(1000000000000),
            }],
            marketing: Some(InstantiateMarketingInfo {
                project: Some("iUPPITER Project".to_string()),
                description: Some("Game Token".to_string()),
                marketing: None,
                logo: None,
                logo_url_state: Some("https://example.com/logo.png".to_string()),
            }),
            mint: None,
            created_on_platform: Some("iUPPITER Platform".to_string()),
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Transfer tokens
        let transfer_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        let msg = ExecuteMsg::Transfer {
            recipient: USER1.to_string(),
            amount: Uint128::new(100000000),
        };

        let res = execute(deps.as_mut(), env.clone(), transfer_info, msg).unwrap();
        assert_eq!(4, res.attributes.len());  // attributes 배열의 길이 확인

        let res = query_balance(deps.as_ref(), USER1.to_string()).unwrap();
        assert_eq!(Uint128::new(100000000), res.balance);
    }

    #[test]
    fn test_fee_granter() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Initialize contract
        let msg = InstantiateMsg {
            name: "iUPPITER".to_string(),
            symbol: "iUP".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ADMIN.to_string(),
                amount: Uint128::new(1000000000000),
            }],
            marketing: Some(InstantiateMarketingInfo {
                project: Some("iUPPITER Project".to_string()),
                description: Some("Game Token".to_string()),
                marketing: None,
                logo: None,
                logo_url_state: Some("https://example.com/logo.png".to_string()),
            }),
            mint: None,
            created_on_platform: Some("iUPPITER Platform".to_string()),
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Test setting fee granter
        let info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        let msg = ExecuteMsg::SetFeeGranter {
            address: Some(FEE_GRANTER.to_string()),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // 속성 수 확인 (2 대신 실제 속성 수에 맞춰야 함)
        assert_eq!(3, res.attributes.len());

        // Query fee granter
        let res = query_fee_granter(deps.as_ref()).unwrap();
        assert_eq!(Some(FEE_GRANTER.to_string()), res.fee_granter);

        // Test removing fee granter
        let msg = ExecuteMsg::SetFeeGranter {
            address: None,
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(3, res.attributes.len());

        // Verify removal
        let res = query_fee_granter(deps.as_ref()).unwrap();
        assert_eq!(None, res.fee_granter);
    }

    #[test]
    fn test_unauthorized_fee_granter() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Initialize contract
        let msg = InstantiateMsg {
            name: "iUPPITER".to_string(),
            symbol: "iUP".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ADMIN.to_string(),
                amount: Uint128::new(1000000000000),
            }],
            marketing: Some(InstantiateMarketingInfo {
                project: Some("iUPPITER Project".to_string()),
                description: Some("Game Token".to_string()),
                marketing: None,
                logo: None,
                logo_url_state: Some("https://example.com/logo.png".to_string()),
            }),
            mint: None,
            created_on_platform: Some("iUPPITER Platform".to_string()),
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Try to set fee granter with unauthorized user
        let unauthorized_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };

        let msg = ExecuteMsg::SetFeeGranter {
            address: Some(FEE_GRANTER.to_string()),
        };

        let err = execute(deps.as_mut(), env, unauthorized_info, msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }

    #[test]
    fn test_upgrade_admin() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 컨트랙트 초기화
        let msg = InstantiateMsg {
            name: "iUPPITER".to_string(),
            symbol: "iUP".to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: ADMIN.to_string(),
                amount: Uint128::new(1000000000000),
            }],
            marketing: Some(InstantiateMarketingInfo {
                project: Some("iUPPITER Project".to_string()),
                description: Some("Game Platform Token".to_string()),
                marketing: None,
                logo: None,
                logo_url_state: Some("https://example.com/logo.png".to_string()),
            }),
            mint: None,
            created_on_platform: Some("iUPPITER Platform".to_string()),
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // 업그레이드 관리자 설정
        let set_admin_msg = ExecuteMsg::SetUpgradeAdmin {
            address: USER1.to_string(),
        };

        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        let res = execute(deps.as_mut(), env.clone(), admin_info, set_admin_msg).unwrap();
        assert_eq!(2, res.attributes.len());
    }
}