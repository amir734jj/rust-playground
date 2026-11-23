use diesel::connection::BoxableConnection;
use diesel::migration::{Migration, MigrationMetadata, MigrationName, MigrationVersion, Result};
use diesel::sqlite::Sqlite;
use sea_query::{SchemaBuilder, SqliteQueryBuilder};
use std::fmt;

pub(super) struct FluentMigration {
    pub version: &'static str,
    pub name: &'static str,
    pub up: fn(&dyn SchemaBuilder) -> Vec<String>,
    pub down: fn(&dyn SchemaBuilder) -> Vec<String>,
}

impl Migration<Sqlite> for FluentMigration {
    fn run(&self, conn: &mut dyn BoxableConnection<Sqlite>) -> Result<()> {
        for sql in (self.up)(&SqliteQueryBuilder) {
            conn.batch_execute(&sql)?;
        }
        Ok(())
    }

    fn revert(&self, conn: &mut dyn BoxableConnection<Sqlite>) -> Result<()> {
        for sql in (self.down)(&SqliteQueryBuilder) {
            conn.batch_execute(&sql)?;
        }
        Ok(())
    }

    fn metadata(&self) -> &dyn MigrationMetadata {
        self
    }

    fn name(&self) -> &dyn MigrationName {
        self
    }
}

impl MigrationMetadata for FluentMigration {}

impl MigrationName for FluentMigration {
    fn version(&self) -> MigrationVersion<'_> {
        self.version.into()
    }
}

impl fmt::Display for FluentMigration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}_{}", self.version, self.name)
    }
}
