use crate::response::ApiResponse;

pub async fn health_check() -> ApiResponse<serde_json::Value> {
    let data = serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    });
    ApiResponse::success(data, "healthy")
}
