use axum::{http::StatusCode, response::IntoResponse, Json};
use import_wp::generate_truncated_uuid;
use import_wp::html_to_mobiledoc;
use mysql::{params, prelude::Queryable, Pool};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct Post {
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
pub struct PostReply {
    id: String,
    title: String,
    slug: String,
    created_at: String,
    updated_at: String,
    author_id: String,
}

pub async fn add_post(Json(post): Json<Post>) -> impl IntoResponse {
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
                "INSERT INTO posts (id, uuid, title, slug, html, lexical, created_at, updated_at, created_by, published_by, published_at, feature_image, email_recipient_filter, status, visibility) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'all', 'published', 'public')",
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
                    &author_id,
                    &post.updated_at,
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
