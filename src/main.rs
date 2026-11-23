use poem::{EndpointExt, Route, Server, listener::TcpListener};
use poem_openapi::OpenApiService;
use rust_api::{api::Api, auth::AppState};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match dotenvy::from_path(".env") {
        Ok(()) => {}
        Err(error) if error.not_found() => {}
        Err(error) => return Err(error.into()),
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => "app.sqlite".to_owned(),
        Err(error) => return Err(error.into()),
    };
    let address: std::net::SocketAddr = match std::env::var("BIND_ADDR") {
        Ok(address) => address.parse()?,
        Err(std::env::VarError::NotPresent) => "127.0.0.1:3000".parse()?,
        Err(error) => return Err(error.into()),
    };
    let secret = match std::env::var("JWT_SECRET") {
        Ok(secret) => secret,
        Err(std::env::VarError::NotPresent) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "JWT_SECRET is not set. Set it to a cryptographically random secret of at least 32 bytes before starting the API.",
            )
            .into());
        }
        Err(std::env::VarError::NotUnicode(_)) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "JWT_SECRET must contain valid Unicode",
            )
            .into());
        }
    };
    let state = AppState::new(rust_api::establish_connection(&database_url)?, secret)?;
    let api = OpenApiService::new(
        Api {
            state: state.clone(),
        },
        "Rust API",
        "0.1.0",
    )
    .server("/");
    let app = Route::new()
        .nest("/docs", api.swagger_ui())
        .at("/openapi.json", api.spec_endpoint())
        .nest("/", api)
        .data(state)
        .before(|mut request| async move {
            let body = request.take_body().into_bytes_limit(16 * 1024).await?;
            request.set_body(body);
            Ok(request)
        });
    println!("API listening on http://{address}; Swagger UI: http://{address}/docs");
    Server::new(TcpListener::bind(address)).run(app).await?;
    Ok(())
}
