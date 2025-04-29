#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{Addr, MessageInfo, Uint128};
    use cw20::Cw20Coin;

    use crate::contract::{execute, instantiate, query_balance, query_transfer_fee};
    use crate::msg::{ExecuteMsg, InstantiateMsg};
    use crate::ContractError;

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

        // 수수료율 1% 설정
        let set_fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("1.0".to_string()),
            fee_collector: Some(FEE_COLLECTOR.to_string()),
        };

        let admin_info = MessageInfo {
            sender: Addr::unchecked(ADMIN),
            funds: vec![],
        };

        // 수수료 설정 실행
        execute(deps.as_mut(), env.clone(), admin_info.clone(), set_fee_msg).unwrap();

        // 수수료 설정 확인
        let fee_response = query_transfer_fee(deps.as_ref()).unwrap();
        assert_eq!(fee_response.transfer_fee, Some("1.000".to_string()));
        assert_eq!(fee_response.fee_collector, Some(FEE_COLLECTOR.to_string()));

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

        // 응답 속성 확인
        assert_eq!(6, res.attributes.len()); // action, from, to, amount, fee_amount, fee_collector
        assert_eq!(res.attributes[0].value, "transfer");
        assert_eq!(res.attributes[3].value, expected_received.to_string());
        assert_eq!(res.attributes[4].value, expected_fee.to_string());
        assert_eq!(res.attributes[5].value, FEE_COLLECTOR);

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
        
        // 수수료율 2.5% 설정
        let set_fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("2.5".to_string()),
            fee_collector: Some(FEE_COLLECTOR.to_string()),
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
        
        // 응답 속성 확인
        assert_eq!(7, res.attributes.len()); // action, from, to, by, amount, fee_amount, fee_collector
        assert_eq!(res.attributes[0].value, "transfer_from");
        assert_eq!(res.attributes[4].value, expected_received.to_string());
        assert_eq!(res.attributes[5].value, expected_fee.to_string());
        assert_eq!(res.attributes[6].value, FEE_COLLECTOR);
        
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
        
        // 너무 작은 수수료율 (0.0005%)
        let invalid_fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("0.0005".to_string()),
            fee_collector: Some(FEE_COLLECTOR.to_string()),
        };
        
        let err = execute(deps.as_mut(), env.clone(), admin_info.clone(), invalid_fee_msg).unwrap_err();
        match err {
            ContractError::InvalidFeePercentage(_) => {}
            _ => panic!("Expected InvalidFeePercentage error"),
        }
        
        // 너무 큰 수수료율 (101%)
        let invalid_fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("101".to_string()),
            fee_collector: Some(FEE_COLLECTOR.to_string()),
        };
        
        let err = execute(deps.as_mut(), env.clone(), admin_info.clone(), invalid_fee_msg).unwrap_err();
        match err {
            ContractError::InvalidFeePercentage(_) => {}
            _ => panic!("Expected InvalidFeePercentage error"),
        }
        
        // 수수료 수취인 없이 수수료율 설정
        let invalid_fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("1.0".to_string()),
            fee_collector: None,
        };
        
        let err = execute(deps.as_mut(), env.clone(), admin_info.clone(), invalid_fee_msg).unwrap_err();
        match err {
            ContractError::InvalidConfig { msg: _ } => {}
            _ => panic!("Expected InvalidConfig error"),
        }
        
        // 권한 없는 사용자가 수수료 설정
        let unauthorized_info = MessageInfo {
            sender: Addr::unchecked(USER1),
            funds: vec![],
        };
        
        let fee_msg = ExecuteMsg::SetTransferFee {
            fee_percentage: Some("1.0".to_string()),
            fee_collector: Some(FEE_COLLECTOR.to_string()),
        };
        
        let err = execute(deps.as_mut(), env, unauthorized_info, fee_msg).unwrap_err();
        match err {
            ContractError::Unauthorized {} => {}
            _ => panic!("Expected Unauthorized error"),
        }
    }
}