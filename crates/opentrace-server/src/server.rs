// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::sync::Arc;

use axum::{
    Json, Router,
    http::StatusCode,
    middleware as axum_middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use serde_json::json;
use tower_http::services::ServeDir;

use crate::{config, db, handlers, middleware, models, state::AppState};

/// OpenTrace management server.
pub struct OpenTraceServer {
    config: config::Config,
    state: AppState,
}

impl OpenTraceServer {
    pub fn new(config: config::Config) -> Result<Self, Box<dyn std::error::Error>> {
        if config.jwt_secret.trim().is_empty() {
            return Err("JWT_SECRET must be set".into());
        }

        let db = db::Database::new(std::path::Path::new(&config.database_path))?;
        let db = Arc::new(db);

        db.ensure_agent_columns()?;
        db.ensure_tracepoint_sink_id_column()?;

        if db.user_count()? == 0 {
            let password = std::env::var("OPENTRACE_BOOTSTRAP_ADMIN_PASSWORD")
                .unwrap_or_else(|_| "admin123".to_string());
            let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
            db.create_user("admin", &password_hash, models::user::UserRole::Admin)?;
            println!("Created bootstrap admin user: admin");
        }

        let state = AppState::new(db, runtime_jwt_secret(&config.jwt_secret));

        Ok(Self { config, state })
    }

    pub async fn run(self) {
        // Protected API routes (auth required) — auth middleware applied ONLY here
        let protected_routes = Router::new()
            .route("/auth/me", get(handlers::auth::me))
            .route("/users", get(handlers::users::list_users))
            .route("/users", post(handlers::users::create_user))
            .route("/users/{id}", delete(handlers::users::delete_user))
            .route("/agents", get(handlers::agents::list_agents))
            .route("/agents", post(handlers::agents::create_agent))
            .route("/agents/{id}", get(handlers::agents::get_agent))
            .route("/agents/{id}", put(handlers::agents::update_agent))
            .route("/agents/{id}", delete(handlers::agents::delete_agent))
            .route("/agents/{id}/sync", post(handlers::agents::sync_agent))
            .route(
                "/agents/{id}/tracer/{name}/start",
                post(handlers::agents::start_tracer),
            )
            .route(
                "/agents/{id}/tracer/{name}/stop",
                post(handlers::agents::stop_tracer),
            )
            .route(
                "/agents/{id}/tracepoints",
                get(handlers::tracepoints::list_tracepoints),
            )
            .route(
                "/agents/{id}/tracepoints",
                post(handlers::tracepoints::create_tracepoint),
            )
            .route(
                "/agents/{agent_id}/tracepoints/{tracepoint_id}",
                put(handlers::tracepoints::update_tracepoint),
            )
            .route(
                "/agents/{agent_id}/tracepoints/{tracepoint_id}",
                delete(handlers::tracepoints::delete_tracepoint),
            )
            .route("/groups", get(handlers::groups::list_groups))
            .route("/groups", post(handlers::groups::create_group))
            .route("/groups/{id}", delete(handlers::groups::delete_group))
            .route("/sinks", get(handlers::sinks::list_sinks))
            .route("/sinks", post(handlers::sinks::create_sink))
            .route("/sinks/{id}", get(handlers::sinks::get_sink))
            .route("/sinks/{id}", put(handlers::sinks::update_sink))
            .route("/sinks/{id}", delete(handlers::sinks::delete_sink))
            .route("/sinks/{id}/bind", post(handlers::sinks::bind_agent))
            .route(
                "/sinks/{id}/bind/{aid}",
                delete(handlers::sinks::unbind_agent),
            )
            .route("/sinks/{id}/agents", get(handlers::sinks::get_sink_agents))
            .route(
                "/agents/{id}/sink-names",
                get(handlers::sinks::get_agent_sinks),
            )
            .route(
                "/sinks/{sid}/agents/{aid}/connect",
                post(handlers::sinks::connect_sink),
            )
            .route(
                "/sinks/{sid}/agents/{aid}/disconnect",
                post(handlers::sinks::disconnect_sink),
            )
            .route(
                "/sinks/{id}/test",
                post(handlers::sinks::test_sink_connectivity),
            )
            .route("/stats", get(handlers::stats::get_stats))
            .route("/agents/{id}/debug/watch", post(handlers::debug::watch))
            .route("/agents/{id}/debug/stop", post(handlers::debug::stop))
            .with_state(self.state.clone())
            .layer(axum_middleware::from_fn_with_state(
                self.state.clone(),
                middleware::auth::auth_middleware,
            ));

        // Public API routes (no auth required)
        let public_routes = Router::new()
            .route("/auth/login", post(handlers::auth::login))
            .with_state(self.state.clone());

        // Combine all API routes under /api prefix
        // Public routes have no auth middleware; protected routes have it
        let api_router = Router::new().nest(
            "/api",
            public_routes
                .merge(protected_routes)
                .fallback(api_not_found),
        );

        // Serve React build from static/dist with SPA fallback
        // CARGO_MANIFEST_DIR is the crate directory at compile time (crates/opentrace-server/)
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dist_dir = manifest_dir.join("static/dist");
        let static_service = if dist_dir.exists() {
            ServeDir::new(dist_dir).not_found_service(get(spa_fallback))
        } else {
            let fallback_dir = manifest_dir.join("static");
            ServeDir::new(fallback_dir).not_found_service(get(spa_fallback_legacy))
        };

        let app = Router::new()
            .merge(api_router)
            .fallback_service(static_service);

        // Start background health check
        crate::health::start(self.state.db.clone());

        println!("OpenTrace Server running on 0.0.0.0:{}", self.config.port);

        let addr = format!("0.0.0.0:{}", self.config.port);
        let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("server exited: {}", e);
        }
    }
}

/// SPA fallback: serve index.html for any non-file route (React Router)
async fn spa_fallback() -> impl IntoResponse {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static/dist/index.html");
    let content = std::fs::read_to_string(path).unwrap_or_else(|_| {
        "<h1>Frontend not built. Run: cd frontend && npm run build</h1>".to_string()
    });
    axum::response::Html(content)
}

/// Legacy SPA fallback for old static dir
async fn spa_fallback_legacy() -> impl IntoResponse {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static/index.html");
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|_| "<h1>No frontend found</h1>".to_string());
    axum::response::Html(content)
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "API endpoint not found" })),
    )
}

fn runtime_jwt_secret(configured_secret: &str) -> String {
    format!("{configured_secret}:{}", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use crate::middleware::auth::{create_token, verify_token};

    use super::runtime_jwt_secret;

    #[test]
    fn runtime_jwt_secret_invalidates_tokens_from_previous_start() {
        let first_start_secret = runtime_jwt_secret("configured-secret");
        let second_start_secret = runtime_jwt_secret("configured-secret");
        let token = create_token(1, "admin", "admin", &first_start_secret).unwrap();

        assert!(verify_token(&token, &first_start_secret).is_ok());
        assert!(verify_token(&token, &second_start_secret).is_err());
    }
}
