use diesel::connection::SimpleConnection;
use diesel::prelude::*;

pub fn run(conn: &mut SqliteConnection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Table::new("users")
        .primary_key("id")
        .text("name", Nullable::No)
        .create(conn)?;

    Table::new("projects")
        .primary_key("id")
        .integer("user_id", Nullable::No)
        .foreign_key("user_id", "users", "id", OnDelete::Cascade)
        .text("name", Nullable::No)
        .index("projects_user_id_idx", vec!["user_id"])
        .create(conn)?;

    Table::new("tokens")
        .primary_key("id")
        .integer("user_id", Nullable::No)
        .foreign_key("user_id", "users", "id", OnDelete::Cascade)
        .text("token_hash", Nullable::No)
        .unique("token_hash")
        .index("tokens_user_id_idx", vec!["user_id"])
        .create(conn)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub enum Nullable {
    Yes,
    No,
}

#[derive(Debug, Clone, Copy)]
pub enum OnDelete {
    Cascade,
    Restrict,
}

struct ColumnDef {
    name: String,
    col_type: String,
    nullable: Nullable,
    unique: bool,
}

struct ForeignKeyDef {
    column: String,
    table: String,
    ref_column: String,
    on_delete: OnDelete,
}

pub struct Table {
    name: String,
    columns: Vec<ColumnDef>,
    primary_key: Option<String>,
    foreign_keys: Vec<ForeignKeyDef>,
    indexes: Vec<(String, Vec<String>)>,
    unique_columns: Vec<String>,
}

impl Table {
    pub fn new(name: &str) -> Self {
        Table {
            name: name.to_string(),
            columns: Vec::new(),
            primary_key: None,
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            unique_columns: Vec::new(),
        }
    }

    pub fn primary_key(mut self, name: &str) -> Self {
        self.primary_key = Some(name.to_string());
        self.columns.push(ColumnDef {
            name: name.to_string(),
            col_type: "INTEGER".to_string(),
            nullable: Nullable::No,
            unique: false,
        });
        self
    }

    pub fn integer(mut self, name: &str, nullable: Nullable) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            col_type: "INTEGER".to_string(),
            nullable,
            unique: false,
        });
        self
    }

    pub fn text(mut self, name: &str, nullable: Nullable) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            col_type: "TEXT".to_string(),
            nullable,
            unique: false,
        });
        self
    }

    pub fn unique(mut self, column: &str) -> Self {
        self.unique_columns.push(column.to_string());
        self
    }

    pub fn foreign_key(
        mut self,
        column: &str,
        table: &str,
        ref_column: &str,
        on_delete: OnDelete,
    ) -> Self {
        self.foreign_keys.push(ForeignKeyDef {
            column: column.to_string(),
            table: table.to_string(),
            ref_column: ref_column.to_string(),
            on_delete,
        });
        self
    }

    pub fn index(mut self, name: &str, columns: Vec<&str>) -> Self {
        self.indexes.push((
            name.to_string(),
            columns.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    pub fn create(
        self,
        conn: &mut SqliteConnection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.name);

        for (i, col) in self.columns.iter().enumerate() {
            if i > 0 {
                sql.push_str(",\n");
            }
            sql.push_str(&format!("    {} {}", col.name, col.col_type));

            if matches!(col.nullable, Nullable::No) {
                sql.push_str(" NOT NULL");
            }

            if let Some(ref pk) = self.primary_key {
                if pk == &col.name {
                    sql.push_str(" PRIMARY KEY");
                }
            }
        }

        for fk in &self.foreign_keys {
            sql.push_str(&format!(
                ",\n    FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE {}",
                fk.column,
                fk.table,
                fk.ref_column,
                match fk.on_delete {
                    OnDelete::Cascade => "CASCADE",
                    OnDelete::Restrict => "RESTRICT",
                }
            ));
        }

        for unique_col in &self.unique_columns {
            sql.push_str(&format!(",\n    UNIQUE ({})", unique_col));
        }

        sql.push_str("\n);");

        conn.batch_execute(&sql)?;

        for (idx_name, columns) in &self.indexes {
            let columns_str = columns.join(", ");
            let idx_sql = format!(
                "CREATE INDEX IF NOT EXISTS {} ON {}({});",
                idx_name, self.name, columns_str
            );
            conn.batch_execute(&idx_sql)?;
        }

        Ok(())
    }
}
