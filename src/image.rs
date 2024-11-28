use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use chrono::{Datelike, Local};
use std::{fs, path::PathBuf};
use webp::{Encoder, WebPMemory};

pub async fn upload_image(mut multipart: Multipart) -> impl IntoResponse {
    while let Some(field) = multipart.next_field().await.unwrap() {
        if let Some(filename) = field.file_name() {
            let content_type = field.content_type().map(|ct| ct.to_string());

            // Verifica se é uma imagem
            if let Some(ct) = content_type {
                if ct.starts_with("image/") {
                    // Lê os bytes do arquivo
                    let bytes = field.bytes().await.unwrap();

                    // Converte a imagem para WebP
                    match convert_to_webp(&bytes) {
                        Ok(webp_bytes) => {
                            // Gera o caminho para salvar a imagem
                            if let Ok(path) = generate_image_path() {
                                // Cria o diretório, se necessário
                                if let Err(e) = fs::create_dir_all(&path.parent().unwrap()) {
                                    eprintln!("Erro ao criar diretório: {}", e);
                                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                                }

                                // Salva o arquivo
                                let file_path = path.to_str().unwrap().to_string();
                                if fs::write(&file_path, webp_bytes.to_vec()).is_ok() {
                                    return (StatusCode::OK, file_path).into_response();
                                } else {
                                    eprintln!("Erro ao salvar o arquivo");
                                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Erro ao converter imagem para WebP: {}", e);
                            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                        }
                    }
                }
            }
        }
    }

    StatusCode::BAD_REQUEST.into_response()
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
