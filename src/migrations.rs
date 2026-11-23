use diesel::connection::SimpleConnection;
use diesel::migration::{Migration, MigrationSource};
use diesel::prelude::*;
use diesel::sqlite::Sqlite;
use diesel_migrations::MigrationHarness;
use sea_query::{SqliteQueryBuilder, Table};

mod fluent;
mod m20261122_000000_create_tables;
mod m20261122_233324_add_authentication;

pub struct Migrations;

impl MigrationSource<Sqlite> for Migrations {
    fn migrations(&self) -> diesel::migration::Result<Vec<Box<dyn Migration<Sqlite>>>> {
        Ok(vec![
            Box::new(fluent::FluentMigration {
                version: "20261122000000",
                name: "create_tables",
                up: m20261122_000000_create_tables::up,
                down: m20261122_000000_create_tables::down,
            }),
            Box::new(fluent::FluentMigration {
                version: "20261122233324",
                name: "add_authentication",
                up: m20261122_233324_add_authentication::up,
                down: m20261122_233324_add_authentication::down,
            }),
        ])
    }
}

pub fn run(conn: &mut SqliteConnection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    conn.transaction(|conn| {
        // Adopt databases created before versioned migrations were introduced.
        let columns = diesel::sql_query("PRAGMA table_info(users)").load::<TableColumn>(conn)?;
        if columns.iter().any(|column| column.name == "name") {
            conn.batch_execute(
                &Table::alter()
                    .table("users")
                    .drop_column("name")
                    .to_string(SqliteQueryBuilder),
            )?;
        }

        conn.run_pending_migrations(Migrations)?;
        Ok(())
    })
}

#[derive(QueryableByName)]
struct TableColumn {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}
