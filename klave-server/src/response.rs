use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub status_code: u16,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            status_code: 200,
        }
    }

    pub fn created(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            status_code: 201,
        }
    }
}

impl ApiResponse<()> {
    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            status_code: status.as_u16(),
        }
    }

    pub fn no_content(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            status_code: 204,
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_string(&self).expect("ApiResponse serialization must not fail");
        (status, [("content-type", "application/json")], body).into_response()
    }
}
