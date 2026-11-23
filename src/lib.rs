pub mod migrations;
pub mod models;
pub mod schema;

use diesel::connection::SimpleConnection;
use diesel::prelude::*;

pub fn establish_connection(
    database_url: &str,
) -> Result<SqliteConnection, Box<dyn std::error::Error + Send + Sync>> {
    let mut connection = SqliteConnection::establish(database_url)?;
    connection.batch_execute("PRAGMA foreign_keys = ON;")?;
    migrations::run(&mut connection)?;
    Ok(connection)
}
