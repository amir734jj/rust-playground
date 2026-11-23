use diesel::prelude::*;

use crate::schema::{projects, tokens, users};

#[derive(Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub age: i32,
    pub firstname: String,
    pub lastname: String,
    pub username: String,
}

#[derive(Debug, Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = projects)]
pub struct Project {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
}

#[derive(Debug, Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = tokens)]
pub struct Token {
    pub id: i32,
    pub user_id: i32,
    pub token_hash: String,
    pub expires_at: i64,
}
