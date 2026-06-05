//! `mailward-api` - the GraphQL server (axum + async-graphql). Serves `/graphql`
//! (POST + GraphiQL on GET), `/graphql/ws` (subscriptions) and the built SPA.

mod graphql;
mod run;
mod settings;
mod state;
mod types;

use async_graphql::http::GraphiQLSource;
use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::Router;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::graphql::build_schema;
use crate::settings::Settings;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    let settings = Settings::from_env();
    let state = AppState::new(settings.clone());
    let schema = build_schema(state);

    let app = Router::new()
        .route(
            "/graphql",
            get(graphiql).post_service(GraphQL::new(schema.clone())),
        )
        .route_service("/graphql/ws", GraphQLSubscription::new(schema))
        // The built SPA (created in Phase 4); harmless 404s until then.
        .fallback_service(ServeDir::new(&settings.web_dir))
        // Permissive CORS so the Vite dev server can call the API in development.
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(&settings.bind_addr)
        .await
        .expect("bind address");
    println!(
        "mailward-api → http://{}/graphql (GraphiQL) · subscriptions at /graphql/ws",
        settings.bind_addr
    );
    axum::serve(listener, app).await.expect("server");
}

/// The GraphiQL playground (served on GET /graphql).
async fn graphiql() -> impl IntoResponse {
    Html(
        GraphiQLSource::build()
            .endpoint("/graphql")
            .subscription_endpoint("/graphql/ws")
            .finish(),
    )
}
