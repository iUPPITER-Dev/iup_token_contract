# CW20 Token Contract with Enhanced Fee Mechanism

아래는 개선된 수수료 메커니즘을 반영한 문서입니다. 새로운 기능과 사용법을 포함하고 있습니다.

## README.md

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
```

## Developing.md

```markdown
# 개발 가이드

이 문서는 고급 수수료 메커니즘을 갖춘 CW20 토큰 컨트랙트 개발 환경 설정 및 작업 방법을 설명합니다.

## 필수 요구 사항

시작하기 전에 다음 항목이 설치되어 있는지 확인하세요:

- [Rust](https://www.rust-lang.org/) 1.60.0+
- [Cargo](https://doc.rust-lang.org/cargo/)
- [Docker](https://www.docker.com/) (선택 사항, 최적화된 빌드용)
- [Go](https://golang.org/) 1.18+ (선택 사항, 블록체인 시뮬레이션용)

## 개발 환경 설정

1. 저장소 복제:
```sh
git clone [저장소-URL]
cd [저장소-이름]
```

2. 개발 도구 설치:
```sh
cargo install cargo-run-script
```

## 테스트

### 단위 테스트 실행

단위 테스트 실행:
```sh
cargo test
```

자세한 출력:
```sh
cargo test -- --nocapture
```

특정 테스트 실행:
```sh
cargo test [테스트_이름]
```

예를 들어, 수수료 관련 테스트 실행:
```sh
cargo test test_transfer_fee
```

### 테스트 환경에서의 주의 사항

수수료 기능을 테스트할 때 다음 사항에 유의하세요:

1. 테스트 환경에서는 수수료 메시지가 자동으로 실행되지 않습니다. 이 문제를 해결하기 위해 `apply_fee_transfers` 헬퍼 함수가 구현되어 있습니다.

2. 여러 수취인을 대상으로 한 수수료 분배를 테스트할 때는 비율의 합이 정확히 1.0(100%)인지 확인하세요.

3. 테스트 환경에서는 bech32 주소 유효성 검사를 건너뛰기 위해 조건부 컴파일(`#[cfg(test)]`)을 사용합니다.

### 블록체인 시뮬레이션으로 테스트

더 현실적인 환경에서 컨트랙트를 테스트하려면 `wasmd`(CosmWasm의 Go 구현)를 사용할 수 있습니다:

1. wasmd 설치:
```sh
git clone https://github.com/CosmWasm/wasmd.git
cd wasmd
make install
```

2. 로컬 블록체인 설정:
```sh
# 체인 초기화
wasmd init mynode --chain-id test-chain

# 계정 추가
wasmd keys add my-account

# 토큰으로 제네시스 계정 추가
wasmd add-genesis-account $(wasmd keys show my-account -a) 10000000000stake,10000000000token

# 제네시스 트랜잭션 생성
wasmd gentx my-account 1000000stake --chain-id test-chain

# 제네시스 트랜잭션 수집
wasmd collect-gentxs

# 블록체인 시작
wasmd start
```

3. 로컬 체인에 컨트랙트 배포 및 테스트(자세한 단계는 Publishing.md 파일 참조).

## 컨트랙트 개발

### 프로젝트 구조

- `src/`: 소스 코드 디렉토리
  - `contract.rs`: 주요 컨트랙트 기능
  - `error.rs`: 오류 정의
  - `msg.rs`: 메시지 정의
  - `state.rs`: 상태 관리
  - `fee.rs`: 수수료 메커니즘 구현

### 수수료 기능 구현

수수료 메커니즘은 주로 다음 파일에 구현되어 있습니다:
- `fee.rs`: 수수료 계산, 검증, 메시지 생성 로직
- `contract.rs`: 다양한 실행 함수에서 수수료 메커니즘 호출
  - `execute_transfer`: 직접 토큰 전송 시 수수료 공제 처리
  - `execute_transfer_from`: 위임된 토큰 전송 시 수수료 공제 처리
  - `execute_send`: 컨트랙트로 토큰 전송 시 수수료 공제 처리
  - `execute_send_from`: 위임된 컨트랙트 전송 시 수수료 공제 처리
  - `execute_set_fee_config`: 수수료 설정 관리

### 테스트 환경의 특별한 고려 사항

테스트 환경에서는 수수료 메시지 처리를 위해 특별한 조건부 컴파일 블록이 있습니다:

```rust
#[cfg(test)]
if !fee_result.fee_amount.is_zero() {
    apply_fee_transfers(deps.storage, &fee_result)?;
}
```

이 코드는 테스트 환경에서만 실행되며, 실제 블록체인에서는 메시지가 자동으로 처리되므로 필요하지 않습니다.

### 새로운 기능 추가

새 기능을 추가할 때:
1. `msg.rs`에 새 메시지 타입 정의
2. 필요한 경우 `state.rs`에 상태 관리 추가
3. `contract.rs`에 기능 로직 구현
4. 새로운 기능에 대한 포괄적인 테스트 추가
5. 변경 사항을 반영하도록 문서 업데이트

## 빌드

### 디버그 빌드

디버그 빌드:
```sh
cargo build
```

### 최적화 빌드

블록체인에 배포하려면 최적화된 빌드가 필요합니다:
```sh
cargo run-script optimize
```

이는 Docker를 사용하여 재현 가능한 빌드 환경을 만들고 최적화된 Wasm 바이너리를 생성합니다.

## 지속적 통합

이 프로젝트는 CI에 GitHub Actions를 사용합니다. 모든 풀 리퀘스트와 메인 브랜치 푸시는 다음을 수행합니다:
1. 단위 테스트 실행
2. 린팅 수행
3. 최적화된 빌드 생성
4. 코드 커버리지 확인

CI 구성은 `.github/workflows/` 디렉토리를 참조하세요.

## 스키마 생성

메시지용 JSON 스키마 파일 생성:
```sh
cargo schema
```

이는 `schema/` 디렉토리에 스키마 파일을 생성하며, 통합 및 프론트엔드 개발에 사용할 수 있습니다.
```

## Publishing.md

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
```

이 문서들은 프로젝트의 개선된 수수료 메커니즘을 반영하며, 개발자와 사용자가 새로운 기능을 이해하고 활용할 수 있도록 도움을 줍니다. 필요에 따라 추가 세부 정보나 예제를 더 포함할 수 있습니다.