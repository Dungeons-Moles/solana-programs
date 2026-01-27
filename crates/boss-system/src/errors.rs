/// Boss system error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BossSystemError {
    /// Stage must be between 1 and 80
    InvalidStage,
    /// No boss found for the given parameters
    BossNotFound,
}

impl std::fmt::Display for BossSystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BossSystemError::InvalidStage => write!(f, "Stage must be between 1 and 80"),
            BossSystemError::BossNotFound => write!(f, "No boss found for the given parameters"),
        }
    }
}

impl std::error::Error for BossSystemError {}
