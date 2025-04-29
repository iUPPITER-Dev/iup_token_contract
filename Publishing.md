# Publishing

This document provides instructions for building, optimizing, and deploying your CW20 token contract with fee mechanism to a Cosmos-based blockchain.

## Preparing for Production

Before publishing your contract, ensure you've thoroughly tested it and addressed all issues:

1. Run all tests:
```sh
cargo test
```

2. Ensure code quality:
```sh
cargo clippy -- -D warnings
```

3. Format your code:
```sh
cargo fmt
```

## Building an Optimized Contract

For deployment on a blockchain, you need an optimized Wasm binary to minimize gas costs:

### Using cargo-run-script (Recommended)

The simplest way to build an optimized contract is using the `cargo-run-script` command:

```sh
cargo run-script optimize
```

This uses Docker to create a reproducible build environment and produces an optimized binary in the `artifacts/` directory.

### Manual Optimization (Alternative)

If you prefer to optimize manually or don't want to use Docker:

1. Build the contract in release mode:
```sh
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
```

2. Optimize with `wasm-opt` from the [binaryen](https://github.com/WebAssembly/binaryen) package:
```sh
wasm-opt -Oz -o ./artifacts/cw20_fee_token.wasm ./target/wasm32-unknown-unknown/release/cw20_fee_token.wasm
```

## Verifying the Optimization

To ensure your optimization was successful:

1. Check the file size (should be much smaller than the debug build):
```sh
ls -lh ./artifacts/cw20_fee_token.wasm
```

2. Verify your binary with `cosmwasm-check`:
```sh
cosmwasm-check ./artifacts/cw20_fee_token.wasm
```

## Storing and Instantiating the Contract

### Using CosmWasm CLI

1. Install the CosmWasm CLI for your target blockchain (e.g., for Juno network):
```sh
npm install -g @cosmjs/cli @cosmjs/stargate @cosmwasm/cli
```

2. Store your contract on the blockchain:
```sh
junod tx wasm store ./artifacts/cw20_fee_token.wasm \
  --from my-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y
```

3. Find your contract code ID:
```sh
junod query wasm list-code
```

4. Instantiate your contract:
```sh
junod tx wasm instantiate $CODE_ID \
  '{"name":"Fee Token","symbol":"FEE","decimals":6,"initial_balances":[{"address":"juno...","amount":"1000000000"}]}' \
  --label "My Fee Token" \
  --from my-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y
```

### Using CosmWasm-JS (For Web Applications)

If you're building a web application:

```javascript
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";

// Connect to wallet
const client = await SigningCosmWasmClient.connectWithSigner(
  rpcEndpoint,
  signer,
  { gasPrice }
);

// Upload contract
const wasm = fs.readFileSync("./artifacts/cw20_fee_token.wasm");
const uploadResult = await client.upload(signerAddress, wasm, "auto");
const codeId = uploadResult.codeId;

// Instantiate contract
const instantiateMsg = {
  name: "Fee Token",
  symbol: "FEE",
  decimals: 6,
  initial_balances: [
    {
      address: signerAddress,
      amount: "1000000000",
    },
  ],
};
const { contractAddress } = await client.instantiate(
  signerAddress,
  codeId,
  instantiateMsg,
  "My Fee Token",
  "auto"
);
```

## After Deployment

### Setting Up Fees

Once your contract is deployed, you can set up fees with:

```sh
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_transfer_fee":{"fee_percentage":"1.0","fee_collector":"juno..."}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y
```

### Checking Contract Information

Query token information:
```sh
junod query wasm contract-state smart $CONTRACT_ADDRESS '{"token_info":{}}'
```

Query fee settings:
```sh
junod query wasm contract-state smart $CONTRACT_ADDRESS '{"transfer_fee":{}}'
```

## Publishing Source Code (Optional but Recommended)

For transparency, consider publishing your contract's source code to:

1. GitHub - Create a repository with your source code
2. Contract verification platforms - Some blockchains offer source code verification services
3. Frontend applications - Provide links to your source code in any frontend application

## License and Attribution

Ensure your published contract correctly attributes any libraries or templates used, and includes appropriate license information.