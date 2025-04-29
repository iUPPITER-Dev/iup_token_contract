# Developing

This document explains how to set up your development environment and start working with the CW20 token contract with fee mechanism.

## Prerequisites

Before you start, make sure you have:

- [Rust](https://www.rust-lang.org/) 1.60.0+
- [Cargo](https://doc.rust-lang.org/cargo/)
- [Docker](https://www.docker.com/) (optional, for optimized builds)
- [Go](https://golang.org/) 1.18+ (optional, for blockchain simulation)

## Development Environment Setup

1. Clone the repository:
```sh
git clone [YOUR-REPOSITORY-URL]
cd [REPOSITORY-NAME]
```

2. Install development tools:
```sh
cargo install cargo-run-script
```

## Testing

### Running Unit Tests

Run unit tests with:
```sh
cargo test
```

For verbose output:
```sh
cargo test -- --nocapture
```

To run a specific test:
```sh
cargo test [TEST_NAME]
```

For example, to run the fee-related tests:
```sh
cargo test test_transfer_fee
```

### Testing with Blockchain Simulation

If you want to test the contract in a more realistic environment, you can use `wasmd` (a Go implementation of CosmWasm):

1. Install wasmd:
```sh
git clone https://github.com/CosmWasm/wasmd.git
cd wasmd
make install
```

2. Set up a local blockchain:
```sh
# Initialize the chain
wasmd init mynode --chain-id test-chain

# Add an account
wasmd keys add my-account

# Add genesis account with tokens
wasmd add-genesis-account $(wasmd keys show my-account -a) 10000000000stake,10000000000token

# Generate a genesis transaction
wasmd gentx my-account 1000000stake --chain-id test-chain

# Collect genesis transactions
wasmd collect-gentxs

# Start the blockchain
wasmd start
```

3. Deploy and test your contract on the local chain (refer to the Publishing.md file for detailed steps).

## Contract Development

### Project Structure

- `src/`: Source code directory
  - `contract.rs`: Main contract functionality
  - `error.rs`: Error definitions
  - `msg.rs`: Message definitions
  - `state.rs`: State management

### Implementing Fee Functionality

The fee mechanism is implemented mainly in:
- `execute_transfer` function: Handles direct token transfers with fee deduction
- `execute_transfer_from` function: Handles delegated token transfers with fee deduction
- `execute_send` function: Handles sending tokens to a contract with fee deduction
- `execute_send_from` function: Handles delegated sending to a contract with fee deduction
- `execute_set_transfer_fee` function: Manages fee configuration

### Adding New Features

When adding new features:
1. Define new message types in `msg.rs`
2. Add state management in `state.rs` if needed
3. Implement the feature's logic in `contract.rs`
4. Add comprehensive tests for the new feature
5. Update documentation to reflect changes

## Building

### Debug Build

For a debug build:
```sh
cargo build
```

### Optimized Build

For deploying on a blockchain, you need an optimized build:
```sh
cargo run-script optimize
```

This uses Docker to create a reproducible build environment and produces an optimized Wasm binary.

## Continuous Integration

This project uses GitHub Actions for CI. Every pull request and push to the main branch will:
1. Run unit tests
2. Perform linting
3. Create an optimized build
4. Check code coverage

See the `.github/workflows/` directory for CI configuration.

## Schema Generation

Generate JSON schema files for messages with:
```sh
cargo schema
```

This creates schema files in the `schema/` directory, which can be used for integrations and frontend development.