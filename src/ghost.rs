use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow("users"))]
#[allow(non_snake_case)]
pub struct User {    
    pub id: String,
    pub name: String,
    pub slug: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow("tags"))]
#[allow(non_snake_case)]
pub struct Tags {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize, Serialize, sqlx::FromRow("posts"))]
#[allow(non_snake_case)]
pub struct Post {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub html: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: String,
}
