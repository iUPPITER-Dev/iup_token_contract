#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Addr, Decimal, MessageInfo, Uint128};
    use cw20::Cw20Coin;

    use crate::contract::{execute, instantiate, query_balance, query_fee_config};
    use crate::error::ContractError;
    use crate::fee::{FeeTokenType, FeeType};
    use crate::msg::{ExecuteMsg, FeeCollectorInput, InstantiateMsg};

    // 테스트 상수 정의 - 수수료 수취자를 별도 주소로 분리
    const CREATOR: &str = "cosmos1vlhe6z8r7al2lyzp7n3j2vl5kd28hhrw0vxmxr";
    const ADMIN: &str = "cosmos1wztmxhufhy98p3n45yqtwhrxlrr9wkg0tt3a3c";
    const USER1: &str = "cosmos1qg9zllptnqvhyvrrvm0j3qjmtc5q6ds7eq0le4";
    const FEE_COLLECTOR: &str = "cosmos1fn9z9vn4k3qwr7vkg0yhzwv2q8h4lu4qsh7qv3"; // 별도 주소로 수정
    const RECIPIENT: &str = "cosmos1vlhe6z8r7al2lyzp7n3j2vl5kd28hhrw0vxmxr";

    #[test]
    fn test_transfer_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 컨트랙트 초기화
        let msg = InstantiateMsg {
            name: "Fee Token".to_string(),
            symbol: "FEE".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: ADMIN.to_string(),
                    amount: Uint128::new(1000000000),
                },
                Cw20Coin {
                    address: USER1.to_string(),
                    amount: Uint128::new(1000000000),
                },
            ],
            marketing: None,
            mint: None,
            created_on_platform: None,
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        // 컨트랙트 인스턴스화
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // 수수료율 1% 설정 (새로운 방식으로)
        let set_fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(1)), // 1%
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(), // 현재 컨트랙트
            },
            collectors: vec![FeeCollectorInput {
                address: FEE_COLLECTOR.to_string(),
                percentage: "1.0".to_string(), // 100%
            }],
            is_active: true,
        };

        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        // 수수료 설정 실행
        execute(deps.as_mut(), env.clone(), admin_info.clone(), set_fee_msg).unwrap();

        // 수수료 설정 확인
        let fee_response = query_fee_config(deps.as_ref()).unwrap();
        assert_eq!(fee_response.is_active, true);
        
        if let FeeType::Percentage(decimal) = fee_response.fee_type {
            assert_eq!(decimal, Decimal::percent(1));
        } else {
            panic!("Expected percentage fee type");
        }

        // USER1이 100 토큰을 전송하면 1% 수수료를 지불해야 함
        let transfer_amount = Uint128::new(100000000); // 100 tokens
        let expected_fee = Uint128::new(1000000);     // 1% = 1 token
        let expected_received = Uint128::new(99000000); // 99 tokens

        let transfer_msg = ExecuteMsg::Transfer {
            recipient: RECIPIENT.to_string(),
            amount: transfer_amount,
        };

        let user_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };

        // 전송 실행
        let res = execute(deps.as_mut(), env.clone(), user_info, transfer_msg).unwrap();

        // 응답 속성 확인 (새로운 방식은 속성이 다를 수 있음)
        assert!(res.attributes.len() >= 4); // 최소 4개 이상 (action, from, to, amount)
        
        // fee_amount 속성 검색
        let fee_amount_attr = res.attributes.iter().find(|attr| attr.key == "fee_amount");
        assert!(fee_amount_attr.is_some());
        assert_eq!(fee_amount_attr.unwrap().value, expected_fee.to_string());

        // 잔액 확인
        let recipient_balance = query_balance(deps.as_ref(), RECIPIENT.to_string()).unwrap();
        assert_eq!(recipient_balance.balance, expected_received);

        let fee_collector_balance = query_balance(deps.as_ref(), FEE_COLLECTOR.to_string()).unwrap();
        assert_eq!(fee_collector_balance.balance, expected_fee);

        let sender_balance = query_balance(deps.as_ref(), USER1.to_string()).unwrap();
        assert_eq!(
            sender_balance.balance, 
            Uint128::new(1000000000) - transfer_amount
        );
    }

    #[test]
    fn test_transfer_from_with_fee() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 컨트랙트 초기화
        let msg = InstantiateMsg {
            name: "Fee Token".to_string(),
            symbol: "FEE".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: ADMIN.to_string(),
                    amount: Uint128::new(1000000000),
                },
                Cw20Coin {
                    address: USER1.to_string(),
                    amount: Uint128::new(1000000000),
                },
            ],
            marketing: None,
            mint: None,
            created_on_platform: None,
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };
        
        // 컨트랙트 인스턴스화
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // USER1이 ADMIN에게 200 토큰 허용
        let approve_msg = ExecuteMsg::IncreaseAllowance {
            spender: ADMIN.to_string(),
            amount: Uint128::new(200000000),
            expires: None,
        };
        
        let user_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };
        
        execute(deps.as_mut(), env.clone(), user_info, approve_msg).unwrap();
        
        // 수수료율 2.5% 설정 (새로운 방식으로)
        let set_fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(2) + Decimal::permille(5)), // 2.5%
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(), // 현재 컨트랙트
            },
            collectors: vec![FeeCollectorInput {
                address: FEE_COLLECTOR.to_string(),
                percentage: "1.0".to_string(), // 100%
            }],
            is_active: true,
        };
        
        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };
        
        // 수수료 설정 실행
        execute(deps.as_mut(), env.clone(), admin_info.clone(), set_fee_msg).unwrap();
        
        // ADMIN이 USER1의 100 토큰을 recipient에게 전송
        let transfer_amount = Uint128::new(100000000); // 100 tokens
        let expected_fee = Uint128::new(2500000);     // 2.5% = 2.5 tokens
        let expected_received = Uint128::new(97500000); // 97.5 tokens
        
        let transfer_from_msg = ExecuteMsg::TransferFrom {
            owner: USER1.to_string(),
            recipient: RECIPIENT.to_string(),
            amount: transfer_amount,
        };
        
        // 전송 실행
        let res = execute(deps.as_mut(), env.clone(), admin_info, transfer_from_msg).unwrap();
        
        // 응답 속성 확인 (새로운 방식은 속성이 다를 수 있음)
        assert!(res.attributes.len() >= 5); // 최소 5개 이상 (action, from, to, by, amount)
        
        // fee_amount 속성 검색
        let fee_amount_attr = res.attributes.iter().find(|attr| attr.key == "fee_amount");
        assert!(fee_amount_attr.is_some());
        assert_eq!(fee_amount_attr.unwrap().value, expected_fee.to_string());
        
        // 잔액 확인
        let recipient_balance = query_balance(deps.as_ref(), RECIPIENT.to_string()).unwrap();
        assert_eq!(recipient_balance.balance, expected_received);
        
        let fee_collector_balance = query_balance(deps.as_ref(), FEE_COLLECTOR.to_string()).unwrap();
        assert_eq!(fee_collector_balance.balance, expected_fee);
        
        let sender_balance = query_balance(deps.as_ref(), USER1.to_string()).unwrap();
        assert_eq!(
            sender_balance.balance, 
            Uint128::new(1000000000) - transfer_amount
        );
    }

    #[test]
    fn test_invalid_fee_settings() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 컨트랙트 초기화
        let msg = InstantiateMsg {
            name: "Fee Token".to_string(),
            symbol: "FEE".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: ADMIN.to_string(),
                    amount: Uint128::new(1000000000),
                },
            ],
            marketing: None,
            mint: None,
            created_on_platform: None,
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };
        
        // 컨트랙트 인스턴스화
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        
        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };
        
        // 수수료 수취인 배열이 비어있는 경우
        let invalid_fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(1)),
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(),
            },
            collectors: vec![], // 비어있는 수취인 배열
            is_active: true,
        };
        
        let err = execute(deps.as_mut(), env.clone(), admin_info.clone(), invalid_fee_msg).unwrap_err();
        match err {
            ContractError::InvalidFeeDistribution {} => {}
            _ => panic!("Expected InvalidFeeDistribution error"),
        }
        
        // 수취인 비율의 합이 100%가 아닌 경우
        let invalid_fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(1)),
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(),
            },
            collectors: vec![
                FeeCollectorInput {
                    address: FEE_COLLECTOR.to_string(),
                    percentage: "0.5".to_string(), // 50%만 할당
                },
            ],
            is_active: true,
        };
        
        let err = execute(deps.as_mut(), env.clone(), admin_info.clone(), invalid_fee_msg).unwrap_err();
        match err {
            ContractError::InvalidFeeDistribution {} => {}
            _ => panic!("Expected InvalidFeeDistribution error"),
        }
        
        // 권한 없는 사용자가 수수료 설정
        let unauthorized_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };
        
        let fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(1)),
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(),
            },
            collectors: vec![
                FeeCollectorInput {
                    address: FEE_COLLECTOR.to_string(),
                    percentage: "1.0".to_string(), // 100%
                },
            ],
            is_active: true,
        };
        
        let err = execute(deps.as_mut(), env, unauthorized_info, fee_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }
    
    #[test]
    fn test_multiple_fee_collectors() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 컨트랙트 초기화
        let msg = InstantiateMsg {
            name: "Fee Token".to_string(),
            symbol: "FEE".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: ADMIN.to_string(),
                    amount: Uint128::new(1000000000),
                },
                Cw20Coin {
                    address: USER1.to_string(),
                    amount: Uint128::new(1000000000),
                },
            ],
            marketing: None,
            mint: None,
            created_on_platform: None,
        };

        let info = MessageInfo {
            sender: Addr::unchecked(CREATOR),
            funds: vec![],
        };

        // 컨트랙트 인스턴스화
        instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // 여러 수취인에게 분배하는 수수료 설정 (총 5%)
        let set_fee_msg = ExecuteMsg::SetFeeConfig {
            fee_type: FeeType::Percentage(Decimal::percent(5)), // 5%
            token_type: FeeTokenType::Cw20 {
                contract_addr: "self".to_string(),
            },
            collectors: vec![
                FeeCollectorInput {
                    address: FEE_COLLECTOR.to_string(), // 첫 번째 수취인 (60%)
                    percentage: "0.6".to_string(),
                },
                FeeCollectorInput {
                    address: ADMIN.to_string(), // 두 번째 수취인 (40%)
                    percentage: "0.4".to_string(),
                },
            ],
            is_active: true,
        };

        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        // 수수료 설정 실행
        execute(deps.as_mut(), env.clone(), admin_info, set_fee_msg).unwrap();

        // USER1이 100 토큰을 전송
        let transfer_amount = Uint128::new(100000000); // 100 tokens
        let _total_fee = Uint128::new(5000000);        // 5% = 5 tokens
        let fee_collector1 = Uint128::new(3000000);   // 60% of 5 = 3 tokens
        let fee_collector2 = Uint128::new(2000000);   // 40% of 5 = 2 tokens
        let expected_received = Uint128::new(95000000); // 95 tokens

        let transfer_msg = ExecuteMsg::Transfer {
            recipient: RECIPIENT.to_string(),
            amount: transfer_amount,
        };

        let user_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };

        // 전송 실행
        execute(deps.as_mut(), env, user_info, transfer_msg).unwrap();

        // 잔액 확인
        let recipient_balance = query_balance(deps.as_ref(), RECIPIENT.to_string()).unwrap();
        assert_eq!(recipient_balance.balance, expected_received);

        // 첫 번째 수취인 잔액
        let fee_collector1_balance = query_balance(deps.as_ref(), FEE_COLLECTOR.to_string()).unwrap();
        assert_eq!(fee_collector1_balance.balance, fee_collector1);

        // 두 번째 수취인 잔액 (초기 1000000000 + 수수료 2000000)
        let fee_collector2_balance = query_balance(deps.as_ref(), ADMIN.to_string()).unwrap();
        assert_eq!(fee_collector2_balance.balance, Uint128::new(1000000000) + fee_collector2);

        // 발신자 잔액
        let sender_balance = query_balance(deps.as_ref(), USER1.to_string()).unwrap();
        assert_eq!(
            sender_balance.balance, 
            Uint128::new(1000000000) - transfer_amount
        );
    }
}