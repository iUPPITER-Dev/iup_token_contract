# Developing.md

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

---

# Development Guide

This document explains how to set up your development environment and work with the CW20 token contract with advanced fee mechanism.

## Prerequisites

Before you start, make sure you have:

- [Rust](https://www.rust-lang.org/) 1.60.0+
- [Cargo](https://doc.rust-lang.org/cargo/)
- [Docker](https://www.docker.com/) (optional, for optimized builds)
- [Go](https://golang.org/) 1.18+ (optional, for blockchain simulation)

## Development Environment Setup

1. Clone the repository:
```sh
git clone [REPOSITORY-URL]
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

For example, to run fee-related tests:
```sh
cargo test test_transfer_fee
```

### Testing Considerations

When testing the fee functionality, note the following:

1. In the test environment, fee messages are not automatically executed. To address this, the helper function `apply_fee_transfers` is implemented.

2. When testing fee distribution to multiple recipients, ensure that the sum of proportions is exactly 1.0 (100%).

3. In the test environment, conditional compilation (`#[cfg(test)]`) is used to skip bech32 address validation.

### Testing with Blockchain Simulation

To test the contract in a more realistic environment, you can use `wasmd` (a Go implementation of CosmWasm):

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

3. Deploy and test your contract on the local chain (see Publishing.md for detailed steps).

## Contract Development

### Project Structure

- `src/`: Source code directory
  - `contract.rs`: Main contract functionality
  - `error.rs`: Error definitions
  - `msg.rs`: Message definitions
  - `state.rs`: State management
  - `fee.rs`: Fee mechanism implementation

### Fee Functionality Implementation

The fee mechanism is implemented mainly in:
- `fee.rs`: Fee calculation, validation, and message generation logic
- `contract.rs`: Fee mechanism calls in various execution functions
  - `execute_transfer`: Handles direct token transfers with fee deduction
  - `execute_transfer_from`: Handles delegated token transfers with fee deduction
  - `execute_send`: Handles sending tokens to a contract with fee deduction
  - `execute_send_from`: Handles delegated sending to a contract with fee deduction
  - `execute_set_fee_config`: Manages fee configuration

### Special Considerations for Test Environment

In the test environment, there's a special conditional compilation block for fee message processing:

```rust
#[cfg(test)]
if !fee_result.fee_amount.is_zero() {
    apply_fee_transfers(deps.storage, &fee_result)?;
}
```

This code runs only in the test environment, as in an actual blockchain the messages are processed automatically.

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
```