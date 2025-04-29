use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{Addr, MessageInfo, Uint128};
use cw20::Cw20Coin;

use crate::contract::{execute, instantiate, query_balance, query_fee_granter};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMarketingInfo, InstantiateMsg};
use crate::state::{EXTENDED_INFO, MARKETING_INFO};

// 테스트 상수 정의
const CREATOR: &str = "cosmos1vlhe6z8r7al2lyzp7n3j2vl5kd28hhrw0vxmxr";
const ADMIN: &str = "cosmos1wztmxhufhy98p3n45yqtwhrxlrr9wkg0tt3a3c";
const USER1: &str = "cosmos1qg9zllptnqvhyvrrvm0j3qjmtc5q6ds7eq0le4";
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

    // 속성 수 확인
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