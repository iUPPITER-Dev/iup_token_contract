use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, Fraction, Response, StdResult, Storage, Uint128, WasmMsg
};
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::state::{FeeConfig, BALANCES, FEE_CONFIG};

/// 수수료 타입 - 퍼센트 또는 고정 금액
#[cw_serde]
pub enum FeeType {
    /// 퍼센트 기반 수수료 (예: "1.5"는 1.5%)
    Percentage(Decimal),
    /// 고정 금액 수수료
    Fixed(Uint128),
}

/// 수수료 토큰 타입
#[cw_serde]
pub enum FeeTokenType {
    /// 네이티브 토큰으로 수수료 수취 (예: XPLA)
    Native { denom: String },
    /// CW20 토큰으로 수수료 수취
    Cw20 { contract_addr: String },
}

/// 수수료 계산 결과 구조체
pub struct FeeCalculationResult {
    pub transfer_amount: Uint128,       // 수수료 차감 후 전송될 금액
    pub fee_amount: Uint128,            // 총 수수료 금액
    pub fee_msgs: Vec<CosmosMsg>,       // 수수료 전송을 위한 메시지들
}

/// 수수료 계산 함수
pub fn calculate_fee(
    deps: Deps,
    amount: Uint128,
    sender: &Addr,
) -> Result<FeeCalculationResult, ContractError> {
    let fee_config = FEE_CONFIG.may_load(deps.storage)?;
    
    // 수수료 설정이 없으면 수수료 없이 전액 전송
    let Some(fee_config) = fee_config else {
        return Ok(FeeCalculationResult {
            transfer_amount: amount,
            fee_amount: Uint128::zero(),
            fee_msgs: vec![],
        });
    };

    // 수수료가 비활성화된 경우
    if !fee_config.is_active {
        return Ok(FeeCalculationResult {
            transfer_amount: amount,
            fee_amount: Uint128::zero(),
            fee_msgs: vec![],
        });
    }

    // 수수료 계산
    let fee_amount = match fee_config.fee_type {
        FeeType::Percentage(percentage) => {
            // 안전한 수치 계산 사용: amount * percentage
            amount.multiply_ratio(percentage.numerator(), percentage.denominator())
        }
        FeeType::Fixed(fixed_amount) => {
            // 고정 금액이 전송 금액보다 큰 경우 오류
            if fixed_amount > amount {
                return Err(ContractError::InsufficientFunds {});
            }
            fixed_amount
        }
    };

    // 수수료가 0이면 수수료 없이 전액 전송
    if fee_amount.is_zero() {
        return Ok(FeeCalculationResult {
            transfer_amount: amount,
            fee_amount: Uint128::zero(),
            fee_msgs: vec![],
        });
    }

    // 전송 금액 계산 (전체 금액 - 수수료)
    let transfer_amount = amount.checked_sub(fee_amount)
        .map_err(|_| ContractError::InvalidAmount {})?;

    // 수수료 분배 메시지 생성
    let fee_msgs = create_fee_distribution_msgs(deps, &fee_config, fee_amount, sender)?;

    Ok(FeeCalculationResult {
        transfer_amount,
        fee_amount,
        fee_msgs,
    })
}

/// 수수료 분배 메시지 생성 함수
fn create_fee_distribution_msgs(
    _deps: Deps,
    fee_config: &FeeConfig,
    total_fee: Uint128,
    _sender: &Addr,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut messages = vec![];
    
    // 수취인이 없으면 빈 메시지 반환
    if fee_config.collectors.is_empty() {
        return Ok(messages);
    }
    
    // 각 수취인에게 비율에 따라 수수료 분배
    for collector in &fee_config.collectors {
        let collector_fee = total_fee.multiply_ratio(
            collector.percentage.numerator(),
            collector.percentage.denominator()
        );
        
        if collector_fee.is_zero() {
            continue;
        }
        
        // 수수료 토큰 유형에 따른 메시지 생성
        let msg = match &fee_config.token_type {
            FeeTokenType::Native { denom } => {
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: collector.address.to_string(),
                    amount: vec![Coin {
                        denom: denom.clone(),
                        amount: collector_fee,
                    }],
                })
            }
            FeeTokenType::Cw20 { contract_addr } => {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.clone(),
                    msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: collector.address.to_string(),
                        amount: collector_fee,
                    })?,
                    funds: vec![],
                })
            }
        };
        
        messages.push(msg);
    }
    
    Ok(messages)
}

/// 수수료 설정 유효성 검사
pub fn validate_fee_config(fee_config: &FeeConfig) -> Result<(), ContractError> {
    // 수수료 수취인 비율 합계가 100%인지 확인
    if !fee_config.collectors.is_empty() {
        let total_percentage = fee_config.collectors
            .iter()
            .fold(Decimal::zero(), |acc, collector| acc + collector.percentage);
        
        // abs() 메서드 대신 범위 비교 사용
        let epsilon = Decimal::percent(1) / Uint128::new(100); // 0.01% 오차 허용
        if total_percentage > Decimal::one() + epsilon || total_percentage < Decimal::one() - epsilon {
            return Err(ContractError::InvalidFeeDistribution {});
        }
    } else if fee_config.is_active {
        // 활성화된 수수료 설정에는 최소 하나의 수취인이 필요
        return Err(ContractError::InvalidFeeDistribution {});
    }
    
    // 고정 수수료 유효성 검사
    if let FeeType::Fixed(amount) = fee_config.fee_type {
        if amount.is_zero() {
            return Err(ContractError::InvalidAmount {});
        }
    }
    
    Ok(())
}

/// 응답에 수수료 속성 추가
pub fn add_fee_attributes(
    response: Response,
    fee_result: &FeeCalculationResult,
    fee_config: Option<&FeeConfig>,
) -> Response {
    let mut response = response;
    
    if !fee_result.fee_amount.is_zero() {
        response = response.add_attribute("fee_amount", fee_result.fee_amount.to_string());
        
        if let Some(config) = fee_config {
            match &config.token_type {
                FeeTokenType::Native { denom } => {
                    response = response.add_attribute("fee_token_type", "native");
                    response = response.add_attribute("fee_token_denom", denom);
                }
                FeeTokenType::Cw20 { contract_addr } => {
                    response = response.add_attribute("fee_token_type", "cw20");
                    response = response.add_attribute("fee_token_address", contract_addr);
                }
            }
            
            // 수취인 정보 추가
            if !config.collectors.is_empty() {
                let collectors_str = config.collectors
                    .iter()
                    .map(|c| format!("{}:{}", c.address, c.percentage))
                    .collect::<Vec<String>>()
                    .join(",");
                    
                response = response.add_attribute("fee_collectors", collectors_str);
            }
        }
    }
    
    response
}

// 테스트 환경에서만 사용되는 수수료 분배 헬퍼 함수
pub fn apply_fee_transfers(
    storage: &mut dyn Storage,
    fee_result: &FeeCalculationResult,
) -> StdResult<()> {
    if fee_result.fee_amount.is_zero() {
        return Ok(());
    }
    
    let fee_config = FEE_CONFIG.may_load(storage)?;
    if let Some(config) = fee_config {
        for collector in &config.collectors {
            let collector_fee = fee_result.fee_amount.multiply_ratio(
                collector.percentage.numerator(),
                collector.percentage.denominator()
            );
            
            if !collector_fee.is_zero() {
                BALANCES.update(
                    storage,
                    &collector.address,
                    |balance| -> StdResult<_> {
                        Ok(balance.unwrap_or_default() + collector_fee)
                    },
                )?;
            }
        }
    }
    
    Ok(())
}