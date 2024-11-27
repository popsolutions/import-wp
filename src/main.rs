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
use mysql::{params, prelude::Queryable, Pool};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

fn generate_truncated_uuid() -> String {
    let uuid = Uuid::new_v4(); // Gera um UUID v4 aleatório
    let hex = uuid.as_simple().to_string(); // Formato sem hífens
    hex[..24].to_string() // Trunca para 24 caracteres
}

#[derive(Deserialize, Serialize)]
struct Tag {
    id: Option<u64>, // Opcional porque o ID será gerado automaticamente
    name: String,
    slug: String,
}

fn html_to_mobiledoc(html: &str) -> Value {
    let document = Html::parse_document(html);
    let p_selector = Selector::parse("p").unwrap();

    let mut children_blocks = vec![];

    for p in document.select(&p_selector) {
        let mut children_text = vec![];

        for text_node in p.text() {
            children_text.push(json!({
                "detail": 0,
                "format": 0,
                "mode": "normal",
                "style": "",
                "text": text_node,
                "type": "extended-text",
                "version": 1
            }));
        }

        children_blocks.push(json!({
            "children": children_text,
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "type": "paragraph",
            "version": 1
        }));
    }

    // Retorna a estrutura final
    json!({
        "root": {
            "children": children_blocks,
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "type": "root",
            "version": 1
        }
    })
}

async fn add_tag(Json(tag): Json<Tag>) -> impl IntoResponse {
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();
    let tag_id = generate_truncated_uuid();
    let result = conn.exec_drop(
        "INSERT INTO tags (id, name, slug, created_at, updated_at, created_by) VALUES (?, ?, ?, NOW(), NOW(), 1)",
        (&tag_id, &tag.name, &tag.slug),
    );

    match result {
        Ok(_) => {
            tracing::info!("add_tag succeeded in inserting new tag");
            // Obtém o ID da tag recém-criada
            let tag_id = conn.last_insert_id();

            let response = Tag {
                id: Some(tag_id),
                name: tag.name,
                slug: tag.slug,
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            tracing::error!("add_tag failed to insert new tag: {:?}", &e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create tag: {}", e),
            )
                .into_response()
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Post {
    id: Option<u64>, // Opcional porque o ID será gerado automaticamente
    title: String,
    slug: String,
    html: String,
    created_at: String,
    updated_at: String,
    author_id: String,
    image_url: Option<String>,
    tags: String,
}

#[derive(Deserialize, Serialize)]
struct PostReply {
    id: String,
    title: String,
    slug: String,
    created_at: String,
    updated_at: String,
    author_id: String,
}

async fn add_post(Json(post): Json<Post>) -> impl IntoResponse {
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();
    let post_id = generate_truncated_uuid();
    let uuid = Uuid::new_v4().to_string();
    let content = html_to_mobiledoc(&post.html);
    let query = "SELECT user_id FROM users_migration WHERE external_id = :external_id";
    let res_author: Option<String> = conn
        .exec_first(query, params! { "external_id" => post.author_id })
        .unwrap_or(None);

    match res_author {
        Some(author_id) => {
            let result = conn.exec_drop(
            "INSERT INTO posts (id, uuid, title, slug, html, lexical, created_at, updated_at, created_by, feature_image, email_recipient_filter) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'all')",
            (
                &post_id,
                &uuid,
                &post.title,
                &post.slug,
                &post.html,
                &content,
                &post.created_at,
                &post.updated_at,
                &author_id,
                &post.image_url.unwrap_or(String::from("")),
            ),
            );

            match result {
                Ok(_) => {
                    tracing::info!("add_post succeeded in inserting new post");
                    let post_authors_id = generate_truncated_uuid();

                    let result_author = conn.exec_drop(
                    "INSERT INTO posts_authors (id, post_id, author_id, sort_order) VALUES (?, ?, ?, ?)",
                    (
                        &post_authors_id,
                        &post_id,
                        &author_id,
                        0,
                    ));
                    match result_author {
                        Ok(_) => {
                            tracing::info!("inserted post author");
                        }
                        Err(e) => {
                            tracing::error!("add_post failed to insert new author: {:?}", &e);
                        }
                    }

                    let tags = post.tags.split(",");
                    tracing::info!("post.tags: {:?}", &post.tags);
                    for tag_item in tags {
                        let query = "SELECT id FROM tags WHERE slug = :slug";
                        let res_tag: Option<String> = conn
                            .exec_first(query, params! { "slug" => tag_item })
                            .unwrap_or(None);

                        match res_tag {
                            Some(tag_id) => {
                                let post_tag = generate_truncated_uuid();
                                let result_tags = conn.exec_drop(
                                "INSERT INTO posts_tags (id, post_id, tag_id, sort_order) VALUES (?, ?, ?, ?)",
                                (&post_tag, &post_id, &tag_id, 0),
                            );
                                match result_tags {
                                    Ok(_) => {
                                        tracing::info!("insert post tag");
                                    }
                                    Err(e) => {
                                        tracing::error!("not insert tag in post {:?}", &e);
                                    }
                                }
                            }
                            None => {
                                tracing::error!("not insert tag in post ");
                            }
                        }
                    }

                    let response = PostReply {
                        id: post_id,
                        title: post.title,
                        slug: post.slug,
                        created_at: post.created_at,
                        updated_at: post.updated_at,
                        author_id: author_id,
                    };
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(_) => {
                    tracing::error!("add_post failed to insert new post");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create post"),
                    )
                        .into_response()
                }
            }
        }
        None => {
            tracing::error!("add_post not found author");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create post: not found author"),
            )
                .into_response()
        }
    }
}

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
    id: String,
    name: String,
    email: String,
}

async fn add_author(Json(user): Json<User>) -> impl IntoResponse {
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();
    let user_id = generate_truncated_uuid();
    let result = conn.exec_drop(
        "INSERT INTO users
            (id, name, email, slug, password, created_at, updated_at, created_by)
        VALUES
            (?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &user_id,
            &user.name,
            &user.email,
            &user.login,
            &user.password,
            &user.created_at,
            &user.created_at,
            1,
        ),
    );

    match result {
        Ok(_) => {
            tracing::info!("add_author sucees to insert new author");
            let user_migration = generate_truncated_uuid();
            let result_mig = conn.exec_drop(
                "INSERT INTO users_migration
                    (id, user_id, external_id)
                VALUES
                    (?, ?, ?)",
                (&user_migration, &user_id, &user.id),
            );
            tracing::info!("add_author sucees to insert new author");
            match result_mig {
                Ok(_) => {
                    let response = Author {
                        id: user_id,
                        name: user.name,
                        email: user.email,
                    };
                    tracing::info!("add_user_mig sucees to insert new author");
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(err) => {
                    tracing::error!("add_user_mig error: {:?}", &err);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create user: {}", err),
                    )
                }
                .into_response(),
            }
        }
        Err(e) => {
            tracing::error!("add_author error: {:?}", &e);
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
        .route("/api/tags", post(add_tag))
        .route("/api/posts", post(add_post))
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
    tracing::info!("health_check started");

    let json_response = serde_json::json!({
        "status": "ok",
        "message": MESSAGE
    });

    Json(json_response)
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
