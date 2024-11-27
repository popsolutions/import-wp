use axum::{http::StatusCode, response::IntoResponse, Json};
use import_wp::generate_truncated_uuid;
use mysql::{prelude::Queryable, Pool};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize, Serialize)]
pub struct User {
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

pub async fn add_author(Json(user): Json<User>) -> impl IntoResponse {
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
