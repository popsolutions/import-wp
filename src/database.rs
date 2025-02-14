use axum::{
    http::StatusCode,
};
use mysql::{Opts, Pool};
use std::env;

pub fn connect_to_database() -> Result<mysql::PooledConn, (StatusCode, String)> {
    let db_url = env::var("DB_URL").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "A variável de ambiente DB_URL não foi definida".to_string(),
        )
    })?;

    let connection_opts = Opts::from_url(&db_url).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "URL de conexão inválida".to_string(),
        )
    })?;

    let pool = Pool::new(connection_opts).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao criar o pool de conexões".to_string(),
        )
    })?;

    let conn = pool.get_conn().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Falha ao obter conexão do pool".to_string(),
        )
    })?;

    Ok(conn)
}


