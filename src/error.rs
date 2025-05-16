use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot set to own account")]
    CannotSetOwnAccount {},

    #[deprecated(note = "Unused. All zero amount checks have been removed")]
    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Allowance is expired")]
    Expired {},

    #[error("No allowance for this account")]
    NoAllowance {},

    #[error("Minting cannot exceed the cap")]
    CannotExceedCap {},

    #[error("Logo binary data exceeds 5KB limit")]
    LogoTooBig {},

    #[error("Invalid xml preamble for SVG")]
    InvalidXmlPreamble {},

    #[error("Invalid png header")]
    InvalidPngHeader {},

    #[error("Invalid expiration value")]
    InvalidExpiration {},

    #[error("Duplicate initial balance addresses")]
    DuplicateInitialBalanceAddresses {},

    // Added new error types
    #[error("Invalid config: {msg}")]
    InvalidConfig { msg: String },

    #[error("Config update not allowed")]
    ConfigUpdateNotAllowed {},

    #[error("Feature not implemented")]
    NotImplemented {},

    #[error("Invalid fee percentage: {0}")]
    InvalidFeePercentage(String),

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Invalid JSON data")]
    InvalidJson {},
    
    #[error("Fee collectors percentages must sum to 100")]
    InvalidFeeDistribution {},
}