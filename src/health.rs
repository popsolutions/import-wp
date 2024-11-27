use axum::{response::IntoResponse, Json};

pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "API Services";
    tracing::info!("health_check started");

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
}
