use axum::{
    response::{IntoResponse, Json, Response},
    http::StatusCode,
};
use mysql::{prelude::Queryable, Opts, Pool};
use std::env;
use serde_json::json;

pub async fn health_check_handler() -> Response {
    tracing::info!("health_check started");

    let db_url = match env::var("DB_URL") {
        Ok(url) => url,
        Err(_) => {
            tracing::info!("error url");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "fail",
                    "message": "A variável de ambiente DB_URL não foi definida"
                })),
            )
                .into_response()
        }
    };

    let connection_opts = match Opts::from_url(&db_url) {
        Ok(opts) => opts,
        Err(_) => {
            tracing::info!("error connection");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "fail",
                    "message": "URL de conexão inválida"
                })),
            )
                .into_response()
        }
    };

    let pool = match Pool::new(connection_opts) {
        Ok(pool) => pool,
        Err(_) => {
            tracing::info!("error pool");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "fail",
                    "message": "Falha ao criar o pool de conexões"
                })),
            )
                .into_response()
        }
    };

    let mut conn = match pool.get_conn() {
        Ok(conn) => conn,
        Err(_) => {
            tracing::info!("error get connection pull");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "fail",
                    "message": "Falha ao obter conexão do pool"
                })),
            )
                .into_response()
        }
    };

    match conn.query::<u8, _>("SELECT 1") {
        Ok(result) => {
            if result.len() > 0 && result[0] == 1 {
                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "ok",
                        "message": "Conexão com o banco de dados MySQL bem-sucedida!"
                    })),
                )
                    .into_response()
            } else {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "status": "fail",
                        "message": "A consulta retornou um resultado inesperado."
                    })),
                )
                    .into_response()
            }
        }
        Err(e) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "fail",
                    "message": format!("Erro ao executar a consulta: {}", e)
                })),
            )
                .into_response()
        }
    }
}

