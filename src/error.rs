use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("no permission to log in to the admin console")]
    ConsoleLoginForbidden,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage location conflicts with another library: {0}")]
    StorageLocationConflict(String),
    #[error("library is temporarily closed")]
    LibraryDisabled(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<&'static str>,
    #[serde(rename = "libraryId", skip_serializing_if = "Option::is_none")]
    library_id: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::ConsoleLoginForbidden => StatusCode::FORBIDDEN,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::StorageLocationConflict(_) => StatusCode::CONFLICT,
            AppError::LibraryDisabled(_) => StatusCode::LOCKED,
            AppError::Database(_)
            | AppError::Jwt(_)
            | AppError::Json(_)
            | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let code = match &self {
            AppError::LibraryDisabled(_) => Some("library_disabled"),
            AppError::StorageLocationConflict(_) => Some("storage_location_conflict"),
            AppError::ConsoleLoginForbidden => Some("console_login_forbidden"),
            _ => None,
        };
        let library_id = match &self {
            AppError::LibraryDisabled(library_id) => Some(library_id.clone()),
            _ => None,
        };
        let body = Json(ErrorBody {
            error: self.to_string(),
            code,
            library_id,
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
