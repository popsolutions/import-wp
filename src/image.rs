use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use base64::decode;

use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
use tracing::{error, info};
use crate::database::connect_to_database;
use mysql::prelude::Queryable;
use serde_json::json;

#[derive(Deserialize, Serialize, Clone)]
pub struct ImageAuthor {
    author_id: String,
    path_image: String,
    base64: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ImagePost {
    post_id: String,
    path_image: String,
    base64: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ImageRequest {
    path_image: String,
    base64: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ImageReply {
    image: String,
}   


fn folder_year(original: String) -> String {
    if let Some(start) = original.find("wp-content/uploads/") {
        let after_prefix = &original[start + "wp-content/uploads/".len()..];

        // Procurar a segunda barra para capturar ano/mês corretamente
        if let Some(end) = after_prefix.find('/') {
            if let Some(second_end) = after_prefix[end + 1..].find('/') {
                let date_part = &after_prefix[..end + second_end + 1];
                let result = format!("/{}/", date_part);
                return String::from(result);
            }
        }
    }
    return original;
}

fn replace_name(original: String) -> String {
    let re = original.replace("wp-content/uploads/", "");
    String::from(re)
}


pub async fn save_image(image:ImageRequest) -> Result<ImageReply, String> {
    tracing::info!("add_image started");
    let image_data = match decode(&image.base64) {
        Ok(data) => data,
        Err(e) => {
            let message_error = format!("Erro ao decodificar base64");
            tracing::error!("Erro ao decodificar base64: {:?}", e);
            return Err(String::from(message_error))
        }
    };

    let image_copy = image.clone();
    let file_name = replace_name(image.path_image);
    info!("Image name: {}", file_name);
    let save_path_file = format!("/opt/ghost/content/images{}", &file_name);
    let save_path = folder_year(String::from(image_copy.path_image));
    let folder_base = String::from("/opt/ghost/content/images");
    let folder_save = format!("{}{}", &folder_base, &save_path);
    info!("save_path: {}", save_path);
    info!("save_path_file: {}", save_path_file);
    // Criar diretório se não existir
    if let Err(e) = fs::create_dir_all(Path::new(&folder_save)) {
        tracing::error!("Erro ao criar diretório de imagens: {:?}", e);
        return Err(String::from("Erro ao criar um diretório da imagem"))
    }

    // Salvar a imagem no sistema de arquivos
    if let Err(e) =
        fs::File::create(&save_path_file).and_then(|mut file| file.write_all(&image_data))
    {
        tracing::error!("Erro ao salvar imagem no disco: {:?}", e);
        return Err(String::from("Erro ao salvar imagem"))
    }

    let image_url = format!("/content/images{}", file_name);
    let image_reply = ImageReply {
        image: image_url.clone(),
    }; 
    info!("image saved in: {}", image_url.as_str());
    Ok(image_reply)
}

pub async fn save_image_author(Json(image_author): Json<ImageAuthor>) -> Response {
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
    let image_request = ImageRequest {
        path_image: String::from(image_author.path_image),
        base64: String::from(image_author.base64),
    };
    match save_image(image_request).await {
        Ok(image_reply) => {
            let image_path_reply = image_reply.image;
            let result = conn.exec_drop(
                "UPDATE users SET profile_image = ? where id = ?",
                (&image_path_reply, &image_author.author_id),
            );

            match result {
                Ok(ress) => {
                    info!("res: {:?}", ress);
                    (
                        StatusCode::CREATED,
                        Json(json!({
                            "image": image_path_reply,
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("Erro ao salvar URL da imagem no banco de dados: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "status": "fail",
                            "message": "Erro ao salvar image no banco de dados"
                        })),
                        )
                        .into_response()
                }
            }

        },
        Err(message) => {
            tracing::info!("error: {}", message);
            return (
                Json(json!({
                    "status": "fail",
                    "message": message
                })),
            )
            .into_response();

        }
    }
}
pub async fn save_image_post(Json(image_post): Json<ImagePost>) -> Response {
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
    let image_request = ImageRequest {
        path_image: String::from(image_post.path_image),
        base64: String::from(image_post.base64),

    };
    match save_image(image_request).await {
        Ok(image_reply) => {
            let image_path_reply = image_reply.image;
            let result = conn.exec_drop(
                "UPDATE posts SET feature_image = ? where id = ?",
                (&image_path_reply, &image_post.post_id),
            );

            match result {
                Ok(ress) => {
                    info!("res: {:?}", ress);
                    (
                        StatusCode::CREATED,
                        Json(json!({
                            "image": image_path_reply,
                        })),
                    )
                        .into_response()
                }
                Err(e) => {
                    error!("Erro ao salvar URL da imagem no banco de dados: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "status": "fail",
                            "message": "Erro ao salvar image no banco de dados"
                        })),
                        )
                        .into_response()
                }
            }

        },
        Err(message) => {
            tracing::info!("error: {}", message);
            return (
                Json(json!({
                    "status": "fail",
                    "message": message
                })),
            )
            .into_response();

        }
    }
}
