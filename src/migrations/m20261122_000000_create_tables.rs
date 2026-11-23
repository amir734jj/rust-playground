use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, SchemaBuilder, Table};

pub(super) fn up(builder: &dyn SchemaBuilder) -> Vec<String> {
    vec![
        Table::create()
            .table("users")
            .if_not_exists()
            .col(ColumnDef::new("id").integer().not_null().primary_key())
            .col(ColumnDef::new("age").integer().not_null())
            .col(ColumnDef::new("firstname").text().not_null())
            .col(ColumnDef::new("lastname").text().not_null())
            .col(ColumnDef::new("username").text().not_null())
            .build_any(builder),
        Table::create()
            .table("projects")
            .if_not_exists()
            .col(ColumnDef::new("id").integer().not_null().primary_key())
            .col(ColumnDef::new("user_id").integer().not_null())
            .col(ColumnDef::new("name").text().not_null())
            .foreign_key(
                ForeignKey::create()
                    .from("projects", "user_id")
                    .to("users", "id")
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .build_any(builder),
        Table::create()
            .table("tokens")
            .if_not_exists()
            .col(ColumnDef::new("id").integer().not_null().primary_key())
            .col(ColumnDef::new("user_id").integer().not_null())
            .col(ColumnDef::new("token_hash").text().not_null().unique_key())
            .foreign_key(
                ForeignKey::create()
                    .from("tokens", "user_id")
                    .to("users", "id")
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .build_any(builder),
    ]
}

pub(super) fn down(builder: &dyn SchemaBuilder) -> Vec<String> {
    ["tokens", "projects", "users"]
        .into_iter()
        .map(|table| Table::drop().table(table).build_any(builder))
        .collect()
}
