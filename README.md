# README.md

```markdown
# CW20 Token Contract with Enhanced Fee Mechanism

이 프로젝트는 표준 CW20 토큰 컨트랙트를 확장하여 고급 트랜잭션 수수료 기능을 구현한 CosmWasm 스마트 컨트랙트입니다. [Cosmos SDK](https://github.com/cosmos/cosmos-sdk) 모듈을 지원하는 모든 체인에 배포할 수 있습니다.

## 주요 기능

### 1. 표준 CW20 토큰 기능
- 토큰 발행 및 전송 기능
- 토큰 소각 기능
- 권한 위임 메커니즘
- 발행자 설정
- 마케팅 정보 설정

### 2. 고급 수수료 메커니즘
- 다양한 수수료 유형 지원 (퍼센티지 또는 고정 금액)
- 네이티브 토큰 또는 CW20 토큰으로 수수료 징수 가능
- 여러 수취인에게 비율에 따라 수수료 분배 가능
- 수수료 활성화/비활성화 기능
- 모든 전송 함수에 적용: `transfer`, `transfer_from`, `send`, `send_from`

## 수수료 설정 방법

관리자는 다음과 같이 수수료를 설정할 수 있습니다:

```rust
// 퍼센티지 기반 수수료 설정 (예: 1%)
let set_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(1)),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(), // 현재 CW20 토큰으로 수수료 지불
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100% 한 계정에 지급
        },
    ],
    is_active: true,
};

// 여러 수취인에게 분배
let multiple_collectors_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(2)),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "collector1_address".to_string(),
            percentage: "0.6".to_string(), // 60%
        },
        FeeCollectorInput {
            address: "collector2_address".to_string(),
            percentage: "0.4".to_string(), // 40%
        },
    ],
    is_active: true,
};

// 네이티브 토큰으로 수수료 징수 (예: 3% XPLA 토큰)
let native_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(3)),
    token_type: FeeTokenType::Native {
        denom: "uxpla".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100%
        },
    ],
    is_active: true,
};

// 고정 금액 수수료 설정
let fixed_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Fixed(Uint128::new(5000000)), // 5 토큰 (6 자리 소수점 기준)
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100%
        },
    ],
    is_active: true,
};
```

수수료 비활성화:
```rust
// 수수료 비활성화
let disable_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::zero()),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(),
        },
    ],
    is_active: false, // 여기서 비활성화
};
```

## 수수료 계산

수수료는 다음과 같이 계산됩니다:

### 퍼센티지 기반:
- 수수료 금액 = 전송 금액 × 수수료율
- 실제 전송 금액 = 전송 금액 - 수수료 금액

예를 들어, 1% 수수료율로 100 토큰을 전송할 경우:
- 수수료 금액: 1 토큰
- 실제 전송 금액: 99 토큰

### 고정 금액:
- 수수료 금액 = 고정 금액
- 실제 전송 금액 = 전송 금액 - 수수료 금액

예를 들어, 5 토큰의 고정 수수료로 100 토큰을 전송할 경우:
- 수수료 금액: 5 토큰
- 실제 전송 금액: 95 토큰

## 컨트랙트 초기화 예시

```rust
let msg = InstantiateMsg {
    name: "Fee Token".to_string(),
    symbol: "FEE".to_string(),
    decimals: 6,
    initial_balances: vec![
        Cw20Coin {
            address: "admin_address".to_string(),
            amount: Uint128::new(1000000000), // 1,000 토큰 (6자리 소수점 기준)
        },
    ],
    marketing: None,
    mint: Some(MinterResponse {
        minter: "minter_address".to_string(),
        cap: None,
    }),
    created_on_platform: Some("platform_name".to_string()), // 선택적 필드
};
```

## 주요 메시지

### 수수료 설정
```rust
ExecuteMsg::SetFeeConfig {
    fee_type: FeeType,
    token_type: FeeTokenType,
    collectors: Vec<FeeCollectorInput>,
    is_active: bool,
}
```

### 수수료 정보 쿼리
```rust
QueryMsg::FeeConfig {}
```

응답:
```rust
pub struct FeeConfigResponse {
    pub fee_type: FeeType,
    pub token_type: FeeTokenType,
    pub collectors: Vec<FeeCollectorResponse>,
    pub is_active: bool,
}
```

## 개발 시작하기

이 프로젝트를 개발하려면 다음 단계를 따르세요:

```sh
# 의존성 설치 및 컴파일
cargo build

# 테스트 실행
cargo test

# 컨트랙트 배포를 위한 최적화 빌드
cargo run-script optimize
```

## 컨트랙트 배포 및 사용

컨트랙트 배포 및 사용에 대한 자세한 정보는 [Developing.md](./Developing.md) 및 [Publishing.md](./Publishing.md) 파일을 참조하세요.

## 라이센스

이 프로젝트는 Apache 2.0 라이센스로 배포됩니다.

---

# CW20 Token Contract with Enhanced Fee Mechanism

This project is a CosmWasm smart contract that extends the standard CW20 token contract with advanced transaction fee functionality. It can be deployed on any chain that supports the [Cosmos SDK](https://github.com/cosmos/cosmos-sdk) module.

## Key Features

### 1. Standard CW20 Token Features
- Token minting and transfer
- Burn functionality
- Allowance mechanism
- Minter configuration
- Marketing information settings

### 2. Advanced Fee Mechanism
- Support for different fee types (percentage or fixed amount)
- Fee collection in native tokens or CW20 tokens
- Fee distribution to multiple recipients based on proportions
- Fee activation/deactivation functionality
- Fee mechanism applied to all transfer functions: `transfer`, `transfer_from`, `send`, `send_from`

## Setting Up Fees

The contract admin can set fees as follows:

```rust
// Set percentage-based fee (e.g., 1%)
let set_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(1)),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(), // Pay fee in current CW20 token
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100% to one account
        },
    ],
    is_active: true,
};

// Distribute to multiple collectors
let multiple_collectors_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(2)),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "collector1_address".to_string(),
            percentage: "0.6".to_string(), // 60%
        },
        FeeCollectorInput {
            address: "collector2_address".to_string(),
            percentage: "0.4".to_string(), // 40%
        },
    ],
    is_active: true,
};

// Collect fee in native token (e.g., 3% XPLA token)
let native_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::percent(3)),
    token_type: FeeTokenType::Native {
        denom: "uxpla".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100%
        },
    ],
    is_active: true,
};

// Set fixed amount fee
let fixed_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Fixed(Uint128::new(5000000)), // 5 tokens (assuming 6 decimal places)
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(), // 100%
        },
    ],
    is_active: true,
};
```

To disable fees:
```rust
// Disable fees
let disable_fee_msg = ExecuteMsg::SetFeeConfig {
    fee_type: FeeType::Percentage(Decimal::zero()),
    token_type: FeeTokenType::Cw20 {
        contract_addr: "self".to_string(),
    },
    collectors: vec![
        FeeCollectorInput {
            address: "fee_collector_address".to_string(),
            percentage: "1.0".to_string(),
        },
    ],
    is_active: false, // Disable here
};
```

## Fee Calculation

Fees are calculated as follows:

### Percentage-based:
- Fee amount = Transfer amount × Fee rate
- Actual transfer amount = Transfer amount - Fee amount

For example, with a 1% fee rate when transferring 100 tokens:
- Fee amount: 1 token
- Actual transfer amount: 99 tokens

### Fixed amount:
- Fee amount = Fixed amount
- Actual transfer amount = Transfer amount - Fee amount

For example, with a fixed fee of 5 tokens when transferring 100 tokens:
- Fee amount: 5 tokens
- Actual transfer amount: 95 tokens

## Contract Initialization Example

```rust
let msg = InstantiateMsg {
    name: "Fee Token".to_string(),
    symbol: "FEE".to_string(),
    decimals: 6,
    initial_balances: vec![
        Cw20Coin {
            address: "admin_address".to_string(),
            amount: Uint128::new(1000000000), // 1,000 tokens (assuming 6 decimal places)
        },
    ],
    marketing: None,
    mint: Some(MinterResponse {
        minter: "minter_address".to_string(),
        cap: None,
    }),
    created_on_platform: Some("platform_name".to_string()), // Optional field
};
```

## Key Messages

### Setting Fees
```rust
ExecuteMsg::SetFeeConfig {
    fee_type: FeeType,
    token_type: FeeTokenType,
    collectors: Vec<FeeCollectorInput>,
    is_active: bool,
}
```

### Querying Fee Information
```rust
QueryMsg::FeeConfig {}
```

Response:
```rust
pub struct FeeConfigResponse {
    pub fee_type: FeeType,
    pub token_type: FeeTokenType,
    pub collectors: Vec<FeeCollectorResponse>,
    pub is_active: bool,
}
```

## Getting Started with Development

To start developing with this project, follow these steps:

```sh
# Install dependencies and compile
cargo build

# Run tests
cargo test

# Optimized build for contract deployment
cargo run-script optimize
```

## Deploying and Using the Contract

For detailed information on how to deploy and use the contract, refer to the [Developing.md](./Developing.md) and [Publishing.md](./Publishing.md) files.

## License

This project is distributed under the Apache 2.0 license.
```