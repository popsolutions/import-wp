use axum::body::Body;
use axum::middleware;
use axum::{
    http::{self, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use mysql::prelude::Queryable;
use mysql::Pool;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize, Serialize)]
struct User {
    id: i32,
    name: String,
    email: String,
    login: String,
    password: String,
    created_at: String,
}

#[derive(Deserialize, Serialize)]
struct Author {
    name: String,
    email: String,
}

async fn add_author(Json(user): Json<User>) -> impl IntoResponse {
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();

    let result = conn.exec_drop(
        "INSERT INTO users (id, name, email, slug, password, created_at, updated_at, created_by) VALUES (?, ?, ?, ?, ?, NOW(), 1)",
        (&user.id, &user.name, &user.email, &user.login, &user.password, &user.created_at),
    );

    match result {
        Ok(_) => {
            tracing::trace!("add_author sucees to insert new author");
            // Obtém o ID do autor recém-criado
            let author_id = conn.last_insert_id();

            let response = Author {
                name: user.name,
                email: user.email,
            };
            tracing::trace!("add_author sucees to insert new author");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("add_author error error: {:?}", &e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create user: {}", e),
            )
        }
        .into_response(),
    }
}

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
    let app = Router::new()
        .route("/api/healthcheck", get(health_check_handler))
        .route("/api/authors", post(add_author))
        .layer(middleware::from_fn(validation_fingerprint));

    println!("🚀 Server started");
    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "API Services";
    tracing::trace!("health_check started");

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
}

async fn validation_fingerprint(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    tracing::trace!("validation_fingerprint started");
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
