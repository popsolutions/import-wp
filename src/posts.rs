use crate::database::connect_to_database;
use axum::{http::StatusCode, response::IntoResponse, Json};
use chrono::NaiveDateTime;
use chrono::Utc;
use import_wp::generate_truncated_uuid;
use import_wp::html_to_mobiledoc;
use mysql::{params, prelude::Queryable, Pool};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
pub struct Post {
    id: Option<u64>, // Opcional porque o ID será gerado automaticamente
    title: String,
    slug: String,
    html: String,
    excerpt: String,
    created_at: String,
    updated_at: String,
    author_id: String,
    image_url: Option<String>,
    meta_title: Option<String>,
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

fn get_meta_title(post: &Post) -> String {
    let meta_title = if let Some(meta_title_string) = &post.meta_title {
        return meta_title_string.to_string();
    } else {
        "".to_string()
    };
    if post.title.len() > 30 {
        meta_title
    } else {
        post.title.to_string()
    }
}

fn insert_post(mut conn: mysql::PooledConn, author_id: String, post: Post) -> impl IntoResponse {
    let post_id = generate_truncated_uuid();
    let uuid = Uuid::new_v4().to_string();
    let content = html_to_mobiledoc(&post.html);
    let image_url_str = match &post.image_url {
        Some(image_url_some) => format!("__GHOST_URL__{}", image_url_some),
        None => String::from(""),
    };
    let result = conn.exec_drop(r#"
        INSERT INTO posts
            (id, uuid, title, slug, html, lexical, created_at, updated_at, created_by, published_by, published_at, feature_image,   type, email_recipient_filter,      status, visibility) VALUES
            ( ?,    ?,     ?,    ?,    ?,       ?,          ?,          ?,          ?,            ?,            ?,             ?, 'post',                  'all', 'published',   'public')
        "#,
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
            &post.created_at,
            &image_url_str,
        ),
    );

    match result {
        Ok(_) => {
            tracing::info!("add_post succeeded in inserting new post");

            let custom_excerpt = conn.exec_drop(
                "UPDATE posts SET custom_excerpt  = ? WHERE id = ?;",
                (&post.excerpt, &post_id),
            );

            match custom_excerpt {
                Ok(_) => {
                    tracing::info!("update post excerpt");
                }
                Err(err) => {
                    tracing::error!("fail to update excerpt: {:?}", &err);
                }
            }

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

            tracing::info!("post.tags: {:?}", &post.tags);

            let mobiledoc_json = format!(
                r#"{{"version":"0.3.1","atoms":[],"cards":[],"markups":[],"sections":[[1,"p",[[0,[],0,"{}"]]]]}}"#,
                &post.html
            );

            let naive_datetime =
                match NaiveDateTime::parse_from_str(&post.created_at, "%Y-%m-%d %H:%M:%S") {
                    Ok(dt) => dt,
                    Err(e) => {
                        tracing::error!("Failed to parse created_at: {:?}", e);
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({
                                "status": "fail",
                                "message": "Invalid created_at format"
                            })),
                        )
                            .into_response();
                    }
                };

            let mobiledoc_revision_id = generate_truncated_uuid();
            let created_at_ts = naive_datetime.timestamp();
            let result_mobiledoc = conn.exec_drop(
                "INSERT INTO mobiledoc_revisions (id, post_id, mobiledoc, created_at, created_at_ts) VALUES (?, ?, ?, ?, ?)",
                (&mobiledoc_revision_id, &post_id, &mobiledoc_json, &post.created_at, &created_at_ts),
            );

            match result_mobiledoc {
                Ok(_) => tracing::info!("inserted mobiledoc revision"),
                Err(e) => tracing::error!("failed to insert mobiledoc revision: {:?}", &e),
            }

            let revision_id = generate_truncated_uuid();

            let post_result_revision = conn.exec_drop(
                "INSERT INTO post_revisions
                (id, post_id, created_at_ts, created_at, lexical, title, post_status, author_id, reason) VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    revision_id,
                    post_id.clone(),
                    created_at_ts,
                    &post.created_at,
                    mobiledoc_json,
                    &post.title.clone(),
                    "published",
                    author_id.clone(),
                    "published",
                ),
            );

            match post_result_revision {
                Ok(_) => tracing::info!("inserted post revision"),
                Err(e) => tracing::error!("failed to insert post revision: {:?}", &e),
            }

             let post_meta_id = generate_truncated_uuid();
             let meta_title = get_meta_title(&post);
             let result_meta = conn.exec_drop(
             "INSERT INTO posts_meta (id, post_id, meta_title, meta_description) VALUES (?, ?, ?, ?)",
             (
                 &post_meta_id,
                 &post_id,
                 &meta_title.clone(),
                 &post.excerpt.clone(),
            ));
            match result_meta {
                Ok(_) => {
                    tracing::info!("inserted post meta");
                }
                Err(err_resuolt_meta) => {
                    tracing::error!("meta failed to insert new author: {:?}", &err_resuolt_meta);
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
        Err(error) => {
            tracing::error!("Error post inset, {:?}", error);
            tracing::error!("add_post failed to insert new post");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create post"),
            )
                .into_response()
        }
    }
}

pub async fn add_post(Json(post): Json<Post>) -> impl IntoResponse {
    tracing::info!("add_post started");
    let mut conn = match connect_to_database() {
        Ok(conn) => conn,
        Err((status, message)) => {
            tracing::info!("error: {}", message);
            return (
                status,
                Json(json!({
                    "status": "fail",
                    "message": message
                })),
            )
                .into_response();
        }
    };

    let query = "SELECT user_id FROM users_migration WHERE external_id = :external_id";
    tracing::info!("search author_id: {:?}", post.author_id);

    let res_author: Option<String> = conn
        .exec_first(query, params! { "external_id" => post.author_id.clone() })
        .unwrap_or(None);

    tracing::info!("search author: {:?}", res_author);

    match res_author {
        Some(author_id) => {
            tracing::info!("author id found: {}", author_id);
            tracing::info!("author id found: {}", author_id);
            insert_post(conn, author_id, post).into_response()
        }
        None => {
            tracing::error!("add_post not found author, set default user");
            insert_post(conn, "1".to_string(), post).into_response()
        }
    }
}
