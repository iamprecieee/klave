use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
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

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status_code = status.as_u16();
        self
    }

    pub fn created(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
            status_code: 201,
        }
    }
    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        let msg = message.into();
        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(internal_error = %msg, "Internal server error occurred");
            Self {
                success: false,
                message: msg,
                data: None,
                status_code: status.as_u16(),
            }
        } else {
            Self {
                success: false,
                message: msg,
                data: None,
                status_code: status.as_u16(),
            }
        }
    }
}

impl ApiResponse<()> {
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
