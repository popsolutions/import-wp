use axum::body::Body;
use axum::middleware;
use axum::{
    http::{self, Request, StatusCode},
    middleware::Next,
    response::Response,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use axum::Json;
use serde_json::json;

mod authors;
mod health;
mod image;
mod posts;
mod tags;
mod database;
use authors::add_author;
use health::health_check_handler;
use image::{save_image_post, save_image_author, save_image_raw};
use posts::add_post;
use tags::add_tag;

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("🌟 importer wordpress data 🌟");
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let _db_url = match env::var("DB_URL") {
        Ok(url) => url,
        Err(_) => {
            tracing::info!("error url");
            panic!("Erro crítico: DB_URL não está configurada");
           
        }
    };
    // Inicializa o pool de conexões do banco de dados
    if let Err((status, message)) = database::init_db_pool() {
        tracing::error!("Falha ao inicializar o pool de banco de dados: {}", message);
        panic!("Erro crítico: Falha ao inicializar o pool de banco de dados");
    }

    let app = Router::new()
        .route("/api/healthcheck", get(health_check_handler))
        .route("/api/authors", post(add_author))
        .route("/api/tags", post(add_tag))
        .route("/api/posts", post(add_post))
        .route("/api/image", post(save_image_raw))
        .route("/api/posts/image", post(save_image_post))
        .route("/api/authors/image", post(save_image_author))
        .layer(middleware::from_fn(validation_fingerprint))
        .layer(middleware::from_fn(error_logging_middleware));
    
    let _api_token = match env::var("API_TOKEN") {
        Ok(url) => url,
        Err(_) => {
            tracing::info!("error url");
            panic!("Erro crítico: API_TOKEN não está configurada");
        }
    };
    println!("🚀 Server started");
    let listener = TcpListener::bind("0.0.0.0:8888").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn validation_fingerprint(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    tracing::info!("validation_fingerprint started");
    let token = match env::var("API_TOKEN") {
        Ok(token) => token,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let expected_auth = format!("Bearer {}", token);

    if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) {
        if auth_header != &expected_auth {
            tracing::error!("validation_fingerprint not valid");
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        tracing::error!("validation_fingerprint not valid");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

async fn error_logging_middleware(req: Request<Body>, next: Next) -> Response {
    let result = next.run(req).await;

    if result.status().is_server_error() {
        tracing::error!(
            "Server error in endpoint: {} - {}",
            result.status(),
            result.status().canonical_reason().unwrap_or("Unknown error")
        );
    } else if result.status() == StatusCode::METHOD_NOT_ALLOWED {
        tracing::error!(
            "Method Not Allowed (405) in endpoint: {} - {}",
            result.status(),
            result.status().canonical_reason().unwrap_or("Unknown error")
        );
    }

    result
}

