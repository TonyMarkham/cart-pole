/// Every fallible step's result: fails with an `AppError`.
pub(crate) type AppResult<T> = std::result::Result<T, crate::error::app_error::AppError>;
