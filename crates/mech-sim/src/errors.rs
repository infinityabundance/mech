use thiserror::Error;

#[derive(Debug, Error)]
pub enum MechSimError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("plotting failure: {0}")]
    Plotting(String),
    #[error("serialization failure: {0}")]
    Serialization(String),
}
