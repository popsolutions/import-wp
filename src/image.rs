use axum::{http::StatusCode, response::IntoResponse, Json};
use base64::decode;

use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
use tracing::{error, info};
use crate::database::connect_to_database;
use mysql::prelude::Queryable;
use serde_json::json;

#[derive(Deserialize, Serialize, Clone)]
pub struct ImagePost {
    post_id: String,
    path_image: String,
    base64: String,
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

pub async fn add_image(Json(image_post): Json<ImagePost>) -> impl IntoResponse {
    tracing::info!("add_image started");
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
    let image_data = match decode(&image_post.base64) {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Erro ao decodificar base64: {:?}", e);
            return (
                StatusCode::BAD_REQUEST,
                "Imagem em base64 inválida".to_string(),
            )
                .into_response();
        }
    };

    let image_copy = image_post.clone();
    let file_name = replace_name(image_post.path_image);
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
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erro ao salvar imagem".to_string(),
        )
            .into_response();
    }

    // Salvar a imagem no sistema de arquivos
    if let Err(e) =
        fs::File::create(&save_path_file).and_then(|mut file| file.write_all(&image_data))
    {
        tracing::error!("Erro ao salvar imagem no disco: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erro ao salvar imagem".to_string(),
        )
            .into_response();
    }

    let image_url = format!("/content/images{}", file_name);
    info!("image saved in: {}", &image_url);
    let result = conn.exec_drop(
        "UPDATE posts SET feature_image = ? where id = ?",
        (&image_url, &image_post.post_id),
    );

    match result {
        Ok(ress) => {
            info!("Imagem salva com sucesso: {}", image_url);
            info!("res: {:?}", ress);
            (
                StatusCode::CREATED,
                format!("Imagem salva com sucesso: {}", image_url),
            )
                .into_response()
        }
        Err(e) => {
            error!("Erro ao salvar URL da imagem no banco de dados: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Erro ao salvar imagem no banco de dados".to_string(),
            )
                .into_response()
        }
    }
}
