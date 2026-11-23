use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use diesel::prelude::*;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use poem::{Request, http::StatusCode};
use poem_openapi::{ApiResponse, Object, SecurityScheme, auth::Bearer, payload::Json};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt::Display,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use crate::{
    api::UserResponse,
    models::User,
    schema::{tokens, users},
};

pub const TOKEN_LIFETIME_SECONDS: i64 = 3600;
const JWT_ISSUER: &str = "rust-api";
const JWT_AUDIENCE: &str = "rust-api";

#[derive(Object)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(ApiResponse)]
pub enum ApiError {
    #[oai(status = 400)]
    BadRequest(Json<ErrorBody>),
    #[oai(status = 401)]
    Unauthorized(Json<ErrorBody>),
    #[oai(status = 404)]
    NotFound(Json<ErrorBody>),
    #[oai(status = 409)]
    Conflict(Json<ErrorBody>),
    #[oai(status = 500)]
    Internal(Json<ErrorBody>),
}

impl ApiError {
    pub fn bad_request(message: &str) -> Self {
        Self::BadRequest(Json(ErrorBody {
            message: message.into(),
        }))
    }

    pub fn unauthorized() -> Self {
        Self::Unauthorized(Json(ErrorBody {
            message: "Invalid credentials or token".into(),
        }))
    }

    pub fn internal(error: impl Display) -> Self {
        eprintln!("Request failed: {error}");
        Self::Internal(Json(ErrorBody {
            message: "Internal server error".into(),
        }))
    }
}

impl From<diesel::result::Error> for ApiError {
    fn from(error: diesel::result::Error) -> Self {
        Self::internal(error)
    }
}

#[derive(Clone)]
pub struct AppState {
    connection: Arc<Mutex<SqliteConnection>>,
    pub(crate) jwt_secret: Arc<[u8]>,
    pub(crate) dummy_password_hash: Arc<str>,
}

impl AppState {
    pub fn new(
        connection: SqliteConnection,
        secret: String,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        if secret.len() < 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "JWT_SECRET must contain at least 32 bytes; use a cryptographically random secret",
            )
            .into());
        }
        let dummy_password_hash = hash_password(Uuid::new_v4().to_string().as_bytes())
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            jwt_secret: Arc::from(secret.into_bytes()),
            dummy_password_hash: Arc::from(dummy_password_hash),
        })
    }

    pub(crate) async fn database<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T, ApiError> + Send + 'static,
    ) -> Result<T, ApiError> {
        let connection = Arc::clone(&self.connection);
        tokio::task::spawn_blocking(move || {
            let mut connection = connection.lock().map_err(ApiError::internal)?;
            operation(&mut connection)
        })
        .await
        .map_err(ApiError::internal)?
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
    iat: i64,
    iss: String,
    aud: String,
    jti: String,
}

pub(crate) fn now() -> Result<i64, ApiError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(ApiError::internal)?;
    i64::try_from(duration.as_secs()).map_err(ApiError::internal)
}

pub(crate) fn hash_password(password: &[u8]) -> Result<String, argon2::password_hash::Error> {
    Argon2::default()
        .hash_password(password, &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
}

pub(crate) fn verify_password(password: &[u8], hash: &str) -> Result<bool, ApiError> {
    let hash = PasswordHash::new(hash).map_err(ApiError::internal)?;
    match Argon2::default().verify_password(password, &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(error) => Err(ApiError::internal(error)),
    }
}

pub(crate) fn issue_token(secret: &[u8], user_id: i32) -> Result<(String, i64), ApiError> {
    let issued_at = now()?;
    let expires_at = issued_at + TOKEN_LIFETIME_SECONDS;
    let token = encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: user_id.to_string(),
            exp: expires_at,
            iat: issued_at,
            iss: JWT_ISSUER.into(),
            aud: JWT_AUDIENCE.into(),
            jti: Uuid::new_v4().to_string(),
        },
        &EncodingKey::from_secret(secret),
    )
    .map_err(ApiError::internal)?;
    Ok((token, expires_at))
}

pub(crate) fn token_hash(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}

pub struct AuthenticatedUser {
    pub user: UserResponse,
    pub(crate) token_hash: String,
}

#[derive(SecurityScheme)]
#[oai(ty = "bearer", bearer_format = "JWT", checker = "check_token")]
pub struct JwtAuth(pub AuthenticatedUser);

async fn check_token(req: &Request, bearer: Bearer) -> poem::Result<AuthenticatedUser> {
    let state = req.data::<AppState>().ok_or_else(|| {
        eprintln!("Request failed: authentication state is missing");
        poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[JWT_ISSUER]);
    validation.set_audience(&[JWT_AUDIENCE]);
    validation.set_required_spec_claims(&["exp", "sub", "iss", "aud"]);
    validation.leeway = 0;
    let claims = match decode::<Claims>(
        &bearer.token,
        &DecodingKey::from_secret(&state.jwt_secret),
        &validation,
    ) {
        Ok(token) => token.claims,
        Err(_) => return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED)),
    };
    let user_id = match claims.sub.parse::<i32>() {
        Ok(id) if id > 0 => id,
        _ => return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED)),
    };
    let current_time =
        now().map_err(|_| poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))?;
    if claims.iat > current_time
        || claims.exp <= current_time
        || Uuid::parse_str(&claims.jti).is_err()
    {
        return Err(poem::Error::from_status(StatusCode::UNAUTHORIZED));
    }
    let hash = token_hash(&bearer.token);
    state
        .database(move |conn| {
            let user = tokens::table
                .inner_join(users::table)
                .filter(tokens::user_id.eq(user_id))
                .filter(tokens::token_hash.eq(&hash))
                .filter(tokens::expires_at.gt(current_time))
                .select(User::as_select())
                .first::<User>(conn)
                .optional()?;
            Ok(user.map(|user| AuthenticatedUser {
                user: user.into(),
                token_hash: hash,
            }))
        })
        .await
        .map_err(|_| poem::Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))?
        .ok_or_else(|| poem::Error::from_status(StatusCode::UNAUTHORIZED))
}
