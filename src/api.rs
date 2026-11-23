use diesel::prelude::*;
use poem_openapi::{ApiResponse, Object, OpenApi, param::Path, payload::Json};

use crate::{
    auth::{self, ApiError, AppState, ErrorBody, JwtAuth, TOKEN_LIFETIME_SECONDS},
    models::{Token, User},
    schema::{tokens, users},
};

#[derive(Object)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub age: i32,
    pub firstname: String,
    pub lastname: String,
}

#[derive(Object)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Object)]
pub struct UserResponse {
    pub id: i32,
    pub age: i32,
    pub firstname: String,
    pub lastname: String,
    pub username: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            age: user.age,
            firstname: user.firstname,
            lastname: user.lastname,
            username: user.username,
        }
    }
}

#[derive(Object)]
pub struct LoginResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Object)]
pub struct TokenResponse {
    pub id: i32,
    pub expires_at: i64,
    pub current: bool,
}

#[derive(ApiResponse)]
pub enum NoContent {
    #[oai(status = 204)]
    Ok,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct LoginUser {
    id: i32,
    password_hash: Option<String>,
}

pub struct Api {
    pub state: AppState,
}

fn validate_credentials(username: &str, password: &str) -> Result<(), ApiError> {
    if !(3..=64).contains(&username.len())
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(ApiError::bad_request(
            "Username must be 3-64 ASCII letters, digits, underscores, or hyphens",
        ));
    }
    if !(12..=128).contains(&password.len()) {
        return Err(ApiError::bad_request("Password must be 12-128 bytes"));
    }
    Ok(())
}

#[OpenApi]
impl Api {
    /// Register a user with a case-sensitive username and an Argon2 password hash.
    #[oai(path = "/auth/register", method = "post")]
    async fn register(
        &self,
        request: Json<RegisterRequest>,
    ) -> Result<Json<UserResponse>, ApiError> {
        let request = request.0;
        validate_credentials(&request.username, &request.password)?;
        if !(0..=150).contains(&request.age) {
            return Err(ApiError::bad_request("Age must be between 0 and 150"));
        }
        let firstname = request.firstname.trim().to_owned();
        let lastname = request.lastname.trim().to_owned();
        if firstname.is_empty()
            || lastname.is_empty()
            || firstname.len() > 100
            || lastname.len() > 100
        {
            return Err(ApiError::bad_request(
                "First and last names must be 1-100 bytes",
            ));
        }
        self.state
            .database(move |conn| {
                let password_hash =
                    auth::hash_password(request.password.as_bytes()).map_err(ApiError::internal)?;
                conn.immediate_transaction(|conn| {
                    if users::table
                        .filter(users::username.eq(&request.username))
                        .select(users::id)
                        .first::<i32>(conn)
                        .optional()?
                        .is_some()
                    {
                        return Err(ApiError::Conflict(Json(ErrorBody {
                            message: "Username is already registered".into(),
                        })));
                    }
                    diesel::insert_into(users::table)
                        .values((
                            users::age.eq(request.age),
                            users::firstname.eq(firstname),
                            users::lastname.eq(lastname),
                            users::username.eq(&request.username),
                            users::password_hash.eq(password_hash),
                        ))
                        .execute(conn)?;
                    let user = users::table
                        .filter(users::username.eq(request.username))
                        .select(User::as_select())
                        .first::<User>(conn)?;
                    Ok(Json(user.into()))
                })
            })
            .await
    }

    /// Log in and create a new one-hour session without revoking other sessions.
    #[oai(path = "/auth/login", method = "post")]
    async fn login(&self, request: Json<LoginRequest>) -> Result<Json<LoginResponse>, ApiError> {
        let request = request.0;
        validate_credentials(&request.username, &request.password)?;
        let secret = self.state.jwt_secret.clone();
        let dummy_hash = self.state.dummy_password_hash.clone();
        self.state
            .database(move |conn| {
                let candidates = users::table
                    .filter(users::username.eq(&request.username))
                    .filter(users::password_hash.is_not_null())
                    .select(LoginUser::as_select())
                    .limit(2)
                    .load::<LoginUser>(conn)?;
                if candidates.len() > 1 {
                    return Err(ApiError::internal(
                        "Multiple credentialed users have the same username",
                    ));
                }
                let user = candidates.into_iter().next();
                let hash = user
                    .as_ref()
                    .and_then(|user| user.password_hash.as_deref())
                    .unwrap_or(&dummy_hash);
                let password_matches = auth::verify_password(request.password.as_bytes(), hash)?;
                let user = user
                    .filter(|_| password_matches)
                    .ok_or_else(ApiError::unauthorized)?;
                let (token, expires_at) = auth::issue_token(&secret, user.id)?;
                diesel::insert_into(tokens::table)
                    .values((
                        tokens::user_id.eq(user.id),
                        tokens::token_hash.eq(auth::token_hash(&token)),
                        tokens::expires_at.eq(expires_at),
                    ))
                    .execute(conn)?;
                Ok(Json(LoginResponse {
                    access_token: token,
                    token_type: "Bearer".into(),
                    expires_in: TOKEN_LIFETIME_SECONDS,
                }))
            })
            .await
    }

    /// Get the authenticated user's profile.
    #[oai(path = "/users/me", method = "get")]
    async fn me(&self, auth: JwtAuth) -> Json<UserResponse> {
        Json(auth.0.user)
    }

    /// List this user's unexpired sessions without exposing tokens or hashes.
    #[oai(path = "/auth/tokens", method = "get")]
    async fn list_tokens(&self, auth: JwtAuth) -> Result<Json<Vec<TokenResponse>>, ApiError> {
        let current_time = auth::now()?;
        self.state
            .database(move |conn| {
                let rows = tokens::table
                    .filter(tokens::user_id.eq(auth.0.user.id))
                    .filter(tokens::expires_at.gt(current_time))
                    .order(tokens::id.asc())
                    .load::<Token>(conn)?;
                Ok(Json(
                    rows.into_iter()
                        .map(|token| TokenResponse {
                            id: token.id,
                            expires_at: token.expires_at,
                            current: token.token_hash == auth.0.token_hash,
                        })
                        .collect(),
                ))
            })
            .await
    }

    /// Revoke one of this user's sessions.
    #[oai(path = "/auth/tokens/:id", method = "delete")]
    async fn revoke_token(&self, auth: JwtAuth, id: Path<i32>) -> Result<NoContent, ApiError> {
        self.state
            .database(move |conn| {
                let deleted = diesel::delete(
                    tokens::table
                        .filter(tokens::id.eq(id.0))
                        .filter(tokens::user_id.eq(auth.0.user.id)),
                )
                .execute(conn)?;
                if deleted == 0 {
                    return Err(ApiError::NotFound(Json(ErrorBody {
                        message: "Token not found".into(),
                    })));
                }
                Ok(NoContent::Ok)
            })
            .await
    }

    /// Revoke only the session used for this request.
    #[oai(path = "/auth/logout", method = "post")]
    async fn logout(&self, auth: JwtAuth) -> Result<NoContent, ApiError> {
        self.state
            .database(move |conn| {
                diesel::delete(
                    tokens::table
                        .filter(tokens::user_id.eq(auth.0.user.id))
                        .filter(tokens::token_hash.eq(auth.0.token_hash)),
                )
                .execute(conn)?;
                Ok(NoContent::Ok)
            })
            .await
    }
}
