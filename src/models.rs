use diesel::prelude::*;
use paste::paste;

use crate::schema::{projects, tokens, users};

macro_rules! define_model {
    (
        $model:ident, $table:ident;
        $($field:ident: $ty:ty),* $(,)?
    ) => {
        paste! {
            #[derive(Debug, Identifiable, Queryable, Selectable)]
            #[diesel(table_name = $table)]
            pub struct $model {
                pub id: i32,
                $($field: $ty),*
            }

            #[derive(Debug, Insertable)]
            #[diesel(table_name = $table)]
            pub struct [<New $model>]<'a> {
                $(pub $field: &'a str),*
            }
        }
    };
}

define_model! {
    User, users;
    name: String
}

#[derive(Debug, Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = projects)]
pub struct Project {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = projects)]
pub struct NewProject<'a> {
    pub user_id: i32,
    pub name: &'a str,
}

#[derive(Debug, Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = tokens)]
pub struct Token {
    pub id: i32,
    pub user_id: i32,
    pub token_hash: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tokens)]
pub struct NewToken<'a> {
    pub user_id: i32,
    pub token_hash: &'a str,
}
