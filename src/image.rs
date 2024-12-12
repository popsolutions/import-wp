use axum::{http::StatusCode, response::IntoResponse, Json};
use base64::decode;
use chrono::Utc;
use chrono::{Datelike, Local};
use mysql::{prelude::Queryable, Pool};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs, io::Write, path::Path};
use tracing::{error, info};
use webp::{Encoder, WebPMemory};

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
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();
    info!("Received request add image");
    // Decodificar base64
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
    let save_path_file = format!("/opt/ghost/content/images/{}", &file_name);
    let save_path = folder_year(String::from(image_copy.path_image));
    // Criar diretório se não existir
    if let Err(e) = fs::create_dir_all(Path::new(&save_path)) {
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

    let image_url = format!("/content/images/{}", file_name);
    info!("image saved in: {}", &image_url);
    let result = conn.exec_drop(
        "UPDATE posts SET feature_image = ? where id = ?",
        (&image_url, &image_post.post_id),
    );

    match result {
        Ok(_) => {
            info!("Imagem salva com sucesso: {}", image_url);
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

/// Converte bytes de uma imagem para o formato WebP.
fn convert_to_webp(image_bytes: &[u8]) -> Result<WebPMemory, String> {
    // Carrega a imagem
    let image = image::load_from_memory(image_bytes)
        .map_err(|e| format!("Erro ao carregar a imagem: {}", e))?;

    // Codifica a imagem como WebP
    let encoder =
        Encoder::from_image(&image).map_err(|e| format!("Erro ao codificar WebP: {}", e))?;
    Ok(encoder.encode(75.0)) // Qualidade do WebP (0 a 100)
}

/// Gera o caminho para salvar a imagem no formato `/opt/ghost/content/images/YYYY/MM/DD-hash.webp`.
fn generate_image_path() -> Result<PathBuf, String> {
    let now = Local::now();
    let hash = uuid::Uuid::new_v4().to_string();
    let path = format!(
        "/opt/ghost/content/images/{}/{}/{}/{}.webp",
        now.year(),
        now.month(),
        now.day(),
        hash
    );
    Ok(PathBuf::from(path))
}
