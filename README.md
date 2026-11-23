# rust-api

The application uses Poem with attribute-driven OpenAPI/Swagger documentation,
Diesel with bundled SQLite, and versioned Rust migrations.

## Running the API

A local, Git-ignored `.env` contains dummy development settings, so you can start
the API with `cargo run`. Its signing secret is public and **must not be used in
production**. The file is not distributed with the source or Docker image.

The application loads an optional `.env` from the working directory before
starting Tokio. Existing environment variables take precedence. A missing file
is allowed; malformed or unreadable files fail startup.

For deployment, or when `.env` is absent, set a cryptographically random signing
secret before starting the server:

```powershell
$bytes = New-Object byte[] 32
[System.Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
$env:JWT_SECRET = [Convert]::ToBase64String($bytes)
cargo run
```

This sets `JWT_SECRET` only in the current PowerShell session. If it is missing,
startup reports `JWT_SECRET is not set`; use `.env` or run the setup commands
before starting the API in a new terminal.

The server listens on `http://127.0.0.1:3000`. Swagger UI is at `/docs`, and the
generated specification is at `/openapi.json`. Set `BIND_ADDR` to change the
address and `DATABASE_URL` to change the default `app.sqlite` path. Relative
database paths are resolved from the working directory.

`JWT_SECRET` is required and must contain at least 32 bytes. Keep it outside source
control and persist it securely across restarts; changing it invalidates existing
JWTs. For deployment, use HTTPS and add login/registration rate limiting at the
reverse proxy before exposing the service publicly. Requests are limited to 16 KiB.

## Docker

Start Docker Desktop with Linux containers enabled. Set `JWT_SECRET` as described
above, then build and run:

```powershell
docker build -t rust-api .
docker run --rm --name rust-api -p 127.0.0.1:3000:3000 --env JWT_SECRET --mount type=volume,source=rust-api-data,target=/data rust-api
```

Open `http://localhost:3000/docs`. OpenAPI uses a relative server URL so Swagger
requests use the same origin, including when running in Docker.

The multi-stage image uses Alpine for both compilation and runtime. Build tools
are installed only in the builder stage for bundled SQLite and cryptography.
The final image contains the executable and Alpine, not the Rust toolchain or
source code. SQLite is bundled. The process runs as a
non-root user, and database data persists in the `rust-api-data` named volume.
Keep the same signing secret to preserve sessions across container restarts.
The build context excludes local databases, build output, and other non-source
files; secrets are provided only at runtime.

## Authentication

| Method | Path | Authentication | Purpose |
| --- | --- | --- | --- |
| POST | `/auth/register` | None | Register with username, password, age, firstname, and lastname |
| POST | `/auth/login` | None | Return a new JWT and its lifetime |
| GET | `/users/me` | Bearer JWT | Get the current user's profile |
| GET | `/auth/tokens` | Bearer JWT | List this user's active sessions |
| DELETE | `/auth/tokens/{id}` | Bearer JWT | Revoke one of this user's sessions |
| POST | `/auth/logout` | Bearer JWT | Revoke only the current session |

Registration reserves case-sensitive usernames using a SQLite write transaction.
Usernames must contain 3-64 ASCII letters, digits, underscores, or hyphens.
Passwords must be 12-128 bytes; they are stored as salted Argon2id hashes. Password
hashes are never included in API responses. Users created before authentication
was added keep a null password hash and cannot log in; register a new username
instead. Password reset and credential setup for legacy users are not implemented.

Each successful login creates a distinct HS256 JWT that expires after one hour.
Users can have multiple active sessions simultaneously. The database stores a
unique SHA-256 hash of each token and its expiration time, never the raw token.
Protected endpoints check the signature, issuer, audience, expiration, and stored
session on every request, so revocation takes effect on subsequent requests.
Expired sessions are excluded from listings but are not automatically deleted.
There is no refresh-token flow.

In Swagger UI, log in, copy `access_token`, and paste it into **Authorize**.
For other clients, use `Authorization: Bearer <access_token>`.

SQLite access and password hashing run on blocking worker threads using one
shared, serialized database connection. Users can own multiple projects and
tokens; deleting a user cascades to both. Use `establish_connection` to enable
foreign keys and apply migrations.

## Database migrations

Insert rows with Diesel column expressions rather than separate `New*` structs, leaving IDs database-generated:

```rust
diesel::insert_into(users::table)
    .values((
        users::age.eq(30),
        users::firstname.eq("Alice"),
        users::lastname.eq("Smith"),
        users::username.eq("alice"),
    ))
    .execute(&mut conn)?;

diesel::insert_into(projects::table)
    .values((projects::user_id.eq(user.id), projects::name.eq("My project")))
    .execute(&mut conn)?;
```

These examples require `diesel::prelude::*` and the corresponding tables from `rust_api::schema`.

Users have required `age`, `firstname`, `lastname`, and `username` fields.
On connection, migrations remove the obsolete `users.name` column from existing
databases, preserving IDs, other user fields, projects, and tokens. Values stored
only in the old `name` column are discarded; back up your database before upgrading.
Existing databases without migration history are adopted without recreating their tables.

Each migration has its own date-stamped Rust file under `src/migrations/`, named
`mYYYYMMDD_HHMMSS_description.rs`, containing both `up(builder)` and `down(builder)`
functions that accept `&dyn SchemaBuilder`.
The initial migration is `src/migrations/m20261122_000000_create_tables.rs` and
uses SeaQuery's fluent schema builder:

```rust
Table::create()
    .table("users")
    .col(ColumnDef::new("age").integer().not_null())
    .build_any(builder)
```

SeaQuery generates SQL; there are no handwritten schema SQL files or custom SQL
generators. `src/migrations/fluent.rs` adapts these definitions to Diesel's
migration harness, which tracks applied versions in `__diesel_schema_migrations`
and runs migrations transactionally.

The SQLite adapter supplies `SqliteQueryBuilder`; the same definitions can generate
PostgreSQL SQL with `PostgresQueryBuilder`. PostgreSQL connections and execution
are not implemented yet. No explicit indexes are created; primary-key and unique
constraints remain, including the unique token hash. Existing indexes in previously
migrated databases are not removed.

For schema changes, add a new date-stamped file and declare its module in
`src/migrations.rs`. Register its functions with the matching `YYYYMMDDHHMMSS`
version (later than existing versions), then rebuild. Do not
edit applied migrations. The original migration version is preserved for
databases that already used the SQL migration. Rolling back the initial migration
drops all three application tables and their data.
