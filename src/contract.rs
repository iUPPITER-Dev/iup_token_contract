#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Order::Ascending;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage, Uint128 
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
     ConfigInfo, ExecuteMsg, FeeGranterResponse, InstantiateMsg, MigrateMsg, QueryMsg, TotalSupplyResponse, TransferFeeResponse
};
use crate::state::{
    ExtendedTokenInfo, MinterData, TokenInfo, ALLOWANCES, ALLOWANCES_SPENDER, BALANCES, CONFIG, EXTENDED_INFO, LOGO, MARKETING_INFO, TOKEN_INFO
};

// Contract name and version
const CONTRACT_NAME: &str = "crates.io:iup-token";
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

// 수수료 계산 헬퍼 함수
pub fn calculate_transfer_amounts(
    _storage: &dyn Storage,
    amount: Uint128,
    config: &ConfigInfo,
) -> Result<(Uint128, Uint128, Option<String>), ContractError> {
    // 수수료 계산
    let fee_amount = if let Some(fee_basis_points) = config.transfer_fee {
        // fee_basis_points는 100 = 1%로 표현됨 (예: 150 = 1.5%)
        println!("Amount: {}, fee_basis_points: {}", amount, fee_basis_points);
        
        // 직접 계산으로 변경
        let fee = amount.u128() * fee_basis_points.u128() / 10000u128;
        println!("Calculated fee: {}", fee);
        Uint128::new(fee)
        
        // 기존 코드
        // amount.multiply_ratio(fee_basis_points, Uint128::new(10000))
    } else {
        Uint128::zero()
    };
    
    println!("Fee amount: {}", fee_amount);
    
    // 실제 전송 금액
    let transfer_amount = amount.checked_sub(fee_amount)
        .map_err(|_| ContractError::InvalidAmount {})?;
    
    println!("Transfer amount: {}", transfer_amount);
    
    // fee_collector를 문자열로 변환하여 반환
    let fee_collector_str = config.fee_collector.as_ref().map(|addr| addr.to_string());
    
    Ok((transfer_amount, fee_amount, fee_collector_str))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    #[cfg(test)]
    println!("Running in test mode");
    
    #[cfg(not(test))]
    println!("Not running in test mode");

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
        },
        ExecuteMsg::SetTransferFee { fee_percentage, fee_collector } => {
            execute_set_transfer_fee(deps, info, fee_percentage, fee_collector)
        },
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
    
    // 설정 로드
    let config = CONFIG.load(deps.storage)?;
    
    // 수수료 계산
    let (transfer_amount, fee_amount, fee_collector_str) = calculate_transfer_amounts(
        deps.storage, amount, &config)?;
    
    // 잔액 체크
    let sender_balance = BALANCES.load(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(ContractError::InsufficientFunds {});
    }
    
    // 발신자 잔액 감소
    BALANCES.update(deps.storage, &info.sender, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_sub(amount)?)
    })?;
    
    // 수신자 잔액 증가
    BALANCES.update(deps.storage, &rcpt_addr, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default() + transfer_amount)
    })?;
    
    // 수수료 수취 계정에 수수료 추가
    if !fee_amount.is_zero() && fee_collector_str.is_some() {
        #[cfg(test)]
        let fee_collector_addr = Addr::unchecked(&fee_collector_str.clone().unwrap());
        
        #[cfg(not(test))]
        let fee_collector_addr = deps.api.addr_validate(&fee_collector_str.clone().unwrap())?;
        
        BALANCES.update(deps.storage, &fee_collector_addr, |balance| -> StdResult<_> {
            let new_balance = balance.unwrap_or_default() + fee_amount;
            Ok(new_balance)
        })?;
    }
    
    let mut response = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("to", recipient)
        .add_attribute("amount", transfer_amount);
    
    if !fee_amount.is_zero() {
        response = response
            .add_attribute("fee_amount", fee_amount)
            .add_attribute("fee_collector", fee_collector_str.unwrap_or_else(|| "None".to_string()));
    }
    
    Ok(response)
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
    
    // 설정 로드
    let config = CONFIG.load(deps.storage)?;
    
    // 수수료 계산
    let (transfer_amount, fee_amount, fee_collector_str) = calculate_transfer_amounts(
        deps.storage, amount, &config)?;
    
    // 잔액 체크
    let sender_balance = BALANCES.load(deps.storage, &info.sender)?;
    if sender_balance < amount {
        return Err(ContractError::InsufficientFunds {});
    }
    
    // 발신자 잔액 감소
    BALANCES.update(deps.storage, &info.sender, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default().checked_sub(amount)?)
    })?;
    
    // 수신자 잔액 증가
    BALANCES.update(deps.storage, &rcpt_addr, |balance| -> StdResult<_> {
        Ok(balance.unwrap_or_default() + transfer_amount)
    })?;
    
    // 수수료 수취 계정에 수수료 추가
    if !fee_amount.is_zero() && fee_collector_str.is_some() {
        let fee_collector_addr = deps.api.addr_validate(&fee_collector_str.clone().unwrap())?;
        BALANCES.update(deps.storage, &fee_collector_addr, |balance| -> StdResult<_> {
            Ok(balance.unwrap_or_default() + fee_amount)
        })?;
    }
    
    let mut response = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", info.sender.to_string())
        .add_attribute("to", contract.clone())
        .add_attribute("amount", transfer_amount)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.to_string(),
                amount: transfer_amount,  // 수수료 차감 후 금액 전달
                msg,
            }
            .into_cosmos_msg(contract)?,
        );
    
    if !fee_amount.is_zero() {
        response = response
            .add_attribute("fee_amount", fee_amount)
            .add_attribute("fee_collector", fee_collector_str.unwrap_or_else(|| "None".to_string()));
    }
    
    Ok(response)
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

// 수수료 설정 함수
pub fn execute_set_transfer_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee_percentage: Option<String>,
    fee_collector: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let extended_info = EXTENDED_INFO.load(deps.storage)?;
    
    // admin 권한 확인
    if info.sender != extended_info.admin {
        if let Some(upgrade_admin) = &config.upgrade_admin {
            if info.sender != *upgrade_admin {
                return Err(ContractError::Unauthorized {});
            }
        } else {
            return Err(ContractError::Unauthorized {});
        }
    }
    
    // 수수료율 문자열을 숫자로 변환하고 검증
    let fee_uint = if let Some(ref fee_str) = fee_percentage {
        // 문자열을 f64로 변환
        let fee_f64 = fee_str.parse::<f64>()
            .map_err(|_| ContractError::InvalidFeePercentage("Not a valid number".to_string()))?;
        
        // 최소 0.001%, 최대 100% 제한
        if fee_f64 < 0.001 {
            return Err(ContractError::InvalidFeePercentage(
                "Fee percentage must be at least 0.001%".to_string()
            ));
        }
        if fee_f64 > 100.0 {
            return Err(ContractError::InvalidFeePercentage(
                "Fee percentage cannot exceed 100%".to_string()
            ));
        }
        
        // 백분율을 내부 표현(10000 기준)으로 변환
        // 예: 1.5% -> 150
        let fee_basis_points = (fee_f64 * 100.0).round() as u128;
        println!("Fee basis points: {}", fee_basis_points); // 디버깅용 로그 추가
        Some(Uint128::new(fee_basis_points))
    } else {
        None
    };
    
    // fee_collector 검증
    let fee_collector_addr = if let Some(ref collector) = fee_collector {
        #[cfg(test)]
        let addr = Addr::unchecked(collector);
        
        #[cfg(not(test))]
        let addr = deps.api.addr_validate(collector)?;
        
        Some(addr)
    } else {
        None
    };
    
    // 수수료가 있는데 수취인이 없으면 오류
    if fee_uint.is_some() && fee_uint != Some(Uint128::zero()) && fee_collector_addr.is_none() {
        return Err(ContractError::InvalidConfig {
            msg: "Fee collector must be set when transfer fee is enabled".to_string()
        });
    }
    
    // 설정 업데이트
    config.transfer_fee = fee_uint;
    config.fee_collector = fee_collector_addr;
    CONFIG.save(deps.storage, &config)?;
    
    let fee_str = fee_percentage.unwrap_or_else(|| "None".to_string());
    let collector_str = fee_collector.unwrap_or_else(|| "None".to_string());
    
    Ok(Response::new()
        .add_attribute("action", "set_transfer_fee")
        .add_attribute("fee_percentage", fee_str)
        .add_attribute("fee_collector", collector_str))
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
        QueryMsg::TransferFee {} => to_json_binary(&query_transfer_fee(deps)?),
    }
}

// 수수료 쿼리 핸들러
pub fn query_transfer_fee(deps: Deps) -> StdResult<TransferFeeResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(TransferFeeResponse {
        transfer_fee: config.transfer_fee.map(|fee| {
            // 내부 저장값(10000 기준)을 백분율 문자열로 변환
            let percentage = fee.u128() as f64 / 100.0;
            format!("{:.3}", percentage)
        }),
        fee_collector: config.fee_collector.map(|addr| addr.to_string()),
    })
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

pub fn query_fee_granter(deps: Deps) -> StdResult<FeeGranterResponse> {
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
