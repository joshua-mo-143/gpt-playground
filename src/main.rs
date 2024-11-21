use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION},
        Method,
    },
    routing::{get, post},
    Router,
};

pub mod endpoints;
pub mod error;
pub mod state;

use shuttle_openai::async_openai::{config::OpenAIConfig, Client};
use shuttle_runtime::DeploymentMetadata;
use state::AppState;
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = std::env::var("DB_URL").expect("DB_URL env var to exist");
    let openai_api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY env var to exist");

    let cfg = OpenAIConfig::new().with_api_key(openai_api_key);
    let openai = Client::config(cfg);

    let state = AppState::new(conn, openai)
        .await
        .map_err(|e| format!("Could not create application state: {e}"))
        .unwrap();

    state.seed().await;

    let origin = "127.0.0.1:8000".to_string();

    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_origin(vec![origin.parse().unwrap()])
        .allow_headers(vec![AUTHORIZATION, ACCEPT])
        .allow_methods(vec![Method::GET, Method::POST]);

    let router = Router::new()
        .route("/api/health", get(endpoints::health_check))
        .route("/api/auth/register", post(endpoints::auth::register))
        .route("/api/auth/login", post(endpoints::auth::login))
        .route(
            "/api/chat/conversations",
            get(endpoints::openai::get_conversation_list),

        .route(
            "/api/chat/conversations/:id",
            get(endpoints::openai::fetch_conversation_messages)
                .post(endpoints::openai::send_message),
        )
        .route("/api/chat/create", post(endpoints::openai::create_chat))
        .layer(cors)
        .nest_service(
            "/",
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html")),
        )
        .with_state(state);

    let tcp = TcpListener::bind("127.0.0.1:8000").await.unwrap();

    axum::serve(tcp, router).await.unwrap();

    Ok(())
}
