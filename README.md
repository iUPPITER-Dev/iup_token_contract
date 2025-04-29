# CW20 Token Contract with Fee Mechanism

This project is a CosmWasm smart contract that extends the standard CW20 token contract with transaction fee functionality. It can be deployed on any chain that supports the [Cosmos SDK](https://github.com/cosmos/cosmos-sdk) module.

## Key Features

### 1. Standard CW20 Token Features
- Token minting and transfer
- Burn functionality
- Allowance mechanism
- Minter configuration
- Marketing information settings

### 2. Fee Mechanism
- Admin can set transfer fee percentage
- Configurable fee collector account
- Fee calculated as percentage of transfer amount
- Fee is deducted from transfer amount before recipient receives tokens
- Fee mechanism applied to all transfer functions: `transfer`, `transfer_from`, `send`, `send_from`

## Setting Up Fees

The contract admin can set fees as follows:

```rust
// Set 1% fee rate
let set_fee_msg = ExecuteMsg::SetTransferFee {
    fee_percentage: Some("1.0".to_string()),
    fee_collector: Some("fee_collector_address".to_string()),
};
```

Fee rate limits:
- Minimum fee rate: 0.001%
- Maximum fee rate: 100%

To remove fee settings:
```rust
// Remove fee settings
let remove_fee_msg = ExecuteMsg::SetTransferFee {
    fee_percentage: None,
    fee_collector: None,
};
```

## Fee Calculation

Fees are calculated during transfers as follows:
- Fee amount = Transfer amount * Fee rate / 100
- Actual transfer amount = Transfer amount - Fee amount

For example, with a 1% fee rate when transferring 100 tokens:
- Fee amount: 1 token
- Actual transfer amount: 99 tokens

## Contract Initialization Example

```rust
let msg = InstantiateMsg {
    name: "Fee Token".to_string(),
    symbol: "FEE".to_string(),
    decimals: 6,
    initial_balances: vec![
        Cw20Coin {
            address: "admin_address".to_string(),
            amount: Uint128::new(1000000000),
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
ExecuteMsg::SetTransferFee {
    fee_percentage: Option<String>,
    fee_collector: Option<String>,
}
```

### Querying Fee Information
```rust
QueryMsg::TransferFee {}
```

Response:
```rust
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TransferFeeResponse {
    pub transfer_fee: Option<String>,
    pub fee_collector: Option<String>,
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