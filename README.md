# rust-api

The application uses Diesel with bundled SQLite and Rust-based migrations. Run `cargo run` to create `app.sqlite`
and apply the embedded migrations. Set `DATABASE_URL` to use another SQLite database path.

Users can own multiple projects and tokens. Deleting a user deletes their projects and tokens. Use `establish_connection` for each database connection so
that SQLite foreign keys are enabled and migrations are applied.

The token table stores a unique `token_hash`, not a plaintext token. Token generation, hashing, and authentication are not implemented; callers must hash tokens before inserting them.

**Migrations are defined in Rust** (`src/migrations.rs`), not SQL files. Each table has its own creation function.
