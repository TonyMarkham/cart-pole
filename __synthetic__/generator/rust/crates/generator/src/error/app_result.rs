use crate::error::app_error::AppError;

/// Every fallible step's result: fails with an `AppError`.
pub(crate) type AppResult<T> = std::result::Result<T, AppError>;
