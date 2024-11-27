use axum::{http::StatusCode, response::IntoResponse, Json};
use import_wp::generate_truncated_uuid;
use mysql::{prelude::Queryable, Pool};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize, Serialize)]
pub struct Tag {
    id: Option<u64>, // Opcional porque o ID será gerado automaticamente
    name: String,
    slug: String,
}

pub async fn add_tag(Json(tag): Json<Tag>) -> impl IntoResponse {
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
