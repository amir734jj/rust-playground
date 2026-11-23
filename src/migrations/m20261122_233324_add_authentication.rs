use sea_query::{ColumnDef, SchemaBuilder, Table};

pub(super) fn up(builder: &dyn SchemaBuilder) -> Vec<String> {
    vec![
        Table::alter()
            .table("users")
            .add_column(ColumnDef::new("password_hash").text())
            .build_any(builder),
        Table::alter()
            .table("tokens")
            .add_column(
                ColumnDef::new("expires_at")
                    .big_integer()
                    .not_null()
                    .default(0),
            )
            .build_any(builder),
    ]
}

pub(super) fn down(builder: &dyn SchemaBuilder) -> Vec<String> {
    vec![
        Table::alter()
            .table("tokens")
            .drop_column("expires_at")
            .build_any(builder),
        Table::alter()
            .table("users")
            .drop_column("password_hash")
            .build_any(builder),
    ]
}
