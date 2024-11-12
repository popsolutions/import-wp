use axum::{
    http,
    http::{StatusCode,Request},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::env; 
use axum::middleware;
use axum::body::Body;

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

use dotenv::dotenv;
use tokio::net::TcpListener;

async fn add_author() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::CREATED)
        .body(Body::from("User created successfully"))
        .unwrap()
}

pub async fn health_check_handler() -> impl IntoResponse {
    const MESSAGE: &str = "API Services";

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
}
    async fn validation_fingerprint(
        req: Request<Body>, // Especificando o tipo Body diretamente
        next: Next,   // Especificando o tipo Body diretamente
    ) -> Result<Response, StatusCode> {
        // Recupera o token do ambiente, retornando 500 se não estiver definido
        let token = match env::var("API_TOKEN") {
            Ok(token) => token,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };
        
        // Define o valor esperado do header de autorização
        let expected_auth = format!("Bearer {}", token);
        
        // Verifica se o header de autorização está presente e se é válido
        if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) {
            if auth_header != &expected_auth {
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    
        // Prossegue para o próximo middleware ou handler
        Ok(next.run(req).await)
    }
    

#[tokio::main]
async fn main() {
    dotenv().ok();
    println!("🌟 REST API Service 🌟");

    let app = Router::new()
        .route("/api/healthcheck", get(health_check_handler))
        .route("/api/authors", post(add_author))
        .layer(middleware::from_fn(validation_fingerprint));
        

    println!("✅ Server started successfully at 0.0.0.0:8080");

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

