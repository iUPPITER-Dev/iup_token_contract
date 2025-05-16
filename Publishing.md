# Publishing.md

```markdown
# 배포 가이드

이 문서는 고급 수수료 메커니즘을 갖춘 CW20 토큰 컨트랙트를 빌드, 최적화 및 Cosmos 기반 블록체인에 배포하기 위한 지침을 제공합니다.

## 프로덕션 준비

컨트랙트를 배포하기 전에 철저히 테스트하고 모든 문제를 해결했는지 확인하세요:

1. 모든 테스트 실행:
```sh
cargo test
```

2. 코드 품질 확인:
```sh
cargo clippy -- -D warnings
```

3. 코드 포맷:
```sh
cargo fmt
```

## 최적화된 컨트랙트 빌드

블록체인에 배포하려면 가스 비용을 최소화하기 위해 최적화된 Wasm 바이너리가 필요합니다:

### cargo-run-script 사용 (권장)

최적화된 컨트랙트를 빌드하는 가장 간단한 방법은 `cargo-run-script` 명령을 사용하는 것입니다:

```sh
cargo run-script optimize
```

이는 Docker를 사용하여 재현 가능한 빌드 환경을 만들고 `artifacts/` 디렉토리에 최적화된 바이너리를 생성합니다.

### 수동 최적화 (대안)

Docker를 사용하지 않고 수동으로 최적화하려면:

1. 릴리스 모드로 컨트랙트 빌드:
```sh
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown
```

2. [binaryen](https://github.com/WebAssembly/binaryen) 패키지의 `wasm-opt`로 최적화:
```sh
wasm-opt -Oz -o ./artifacts/cw20_fee_token.wasm ./target/wasm32-unknown-unknown/release/cw20_fee_token.wasm
```

## 최적화 확인

최적화가 성공적인지 확인하려면:

1. 파일 크기 확인 (디버그 빌드보다 훨씬 작아야 함):
```sh
ls -lh ./artifacts/cw20_fee_token.wasm
```

2. `cosmwasm-check`로 바이너리 확인:
```sh
cosmwasm-check ./artifacts/cw20_fee_token.wasm
```

## 컨트랙트 저장 및 인스턴스화

### CosmWasm CLI 사용

1. 대상 블록체인용 CosmWasm CLI 설치(예: Juno 네트워크):
```sh
npm install -g @cosmjs/cli @cosmjs/stargate @cosmwasm/cli
```

2. 블록체인에 컨트랙트 저장:
```sh
junod tx wasm store ./artifacts/cw20_fee_token.wasm \
  --from my-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y
```

3. 컨트랙트 코드 ID 찾기:
```sh
junod query wasm list-code
```

4. 컨트랙트 인스턴스화:
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

### CosmWasm-JS 사용 (웹 애플리케이션용)

웹 애플리케이션을 구축하는 경우:

```javascript
import { SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";

// 지갑 연결
const client = await SigningCosmWasmClient.connectWithSigner(
  rpcEndpoint,
  signer,
  { gasPrice }
);

// 컨트랙트 업로드
const wasm = fs.readFileSync("./artifacts/cw20_fee_token.wasm");
const uploadResult = await client.upload(signerAddress, wasm, "auto");
const codeId = uploadResult.codeId;

// 컨트랙트 인스턴스화
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

## 배포 후

### 수수료 설정

컨트랙트가 배포된 후, 새로운 방식으로 수수료를 설정할 수 있습니다:

```sh
# 1% 수수료 설정, 한 명의 수취인
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"percentage":"0.01"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno...","percentage":"1.0"}],"is_active":true}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y

# 여러 수취인에게 수수료 분배
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"percentage":"0.02"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno1...","percentage":"0.7"},{"address":"juno2...","percentage":"0.3"}],"is_active":true}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y

# 고정 금액 수수료 설정
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"fixed":"500000"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno...","percentage":"1.0"}],"is_active":true}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y
```

### 컨트랙트 정보 확인

토큰 정보 쿼리:
```sh
junod query wasm contract-state smart $CONTRACT_ADDRESS '{"token_info":{}}'
```

수수료 설정 쿼리:
```sh
junod query wasm contract-state smart $CONTRACT_ADDRESS '{"fee_config":{}}'
```

## 소스 코드 게시 (선택 사항이지만 권장됨)

투명성을 위해 컨트랙트의 소스 코드 게시를 고려하세요:

1. GitHub - 소스 코드로 저장소 만들기
2. 컨트랙트 검증 플랫폼 - 일부 블록체인은 소스 코드 검증 서비스 제공
3. 프론트엔드 애플리케이션 - 프론트엔드 애플리케이션에서 소스 코드에 대한 링크 제공

## 라이센스 및 기여

게시된 컨트랙트가 사용된 라이브러리나 템플릿을 올바르게 기여하고, 적절한 라이센스 정보를 포함하는지 확인하세요.

---

# Publishing Guide

This document provides instructions for building, optimizing, and deploying your CW20 token contract with advanced fee mechanism to a Cosmos-based blockchain.

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

Once your contract is deployed, you can set up fees with the new approach:

```sh
# Set 1% fee with a single collector
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"percentage":"0.01"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno...","percentage":"1.0"}],"is_active":true}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y

# Distribute fees to multiple collectors
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"percentage":"0.02"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno1...","percentage":"0.7"},{"address":"juno2...","percentage":"0.3"}],"is_active":true}}' \
  --from admin-account \
  --chain-id juno-1 \
  --gas-prices 0.025ujuno \
  --gas auto \
  --gas-adjustment 1.3 \
  -y

# Set fixed amount fee
junod tx wasm execute $CONTRACT_ADDRESS \
  '{"set_fee_config":{"fee_type":{"fixed":"500000"},"token_type":{"cw20":{"contract_addr":"'$CONTRACT_ADDRESS'"}},"collectors":[{"address":"juno...","percentage":"1.0"}],"is_active":true}}' \
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
junod query wasm contract-state smart $CONTRACT_ADDRESS '{"fee_config":{}}'
```

## Publishing Source Code (Optional but Recommended)

For transparency, consider publishing your contract's source code to:

1. GitHub - Create a repository with your source code
2. Contract verification platforms - Some blockchains offer source code verification services
3. Frontend applications - Provide links to your source code in any frontend application

## License and Attribution

Ensure your published contract correctly attributes any libraries or templates used, and includes appropriate license information.
```