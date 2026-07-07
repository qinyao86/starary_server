mod access;
mod activity;
mod assets;
mod auth_routes;
mod health;
mod libraries;
mod members;
mod setup;
mod storage_roots;
pub(crate) mod users;

use crate::state::AppState;
use axum::{
    response::Redirect,
    routing::{get, patch, post},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

pub fn router(state: AppState) -> Router {
    let admin_assets_dir = state.config.resolved_admin_assets_dir();
    let admin_index = admin_assets_dir.join("index.html");
    let admin_service =
        ServeDir::new(admin_assets_dir).not_found_service(ServeFile::new(admin_index));

    Router::new()
        .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
        .nest_service("/admin/", admin_service)
        .route("/health", get(health::health))
        .route("/api/v1/server/info", get(health::server_info))
        .route("/api/v1/setup/status", get(setup::setup_status))
        .route("/api/v1/setup/owner", post(setup::create_owner))
        .route("/api/v1/auth/login", post(auth_routes::login))
        .route("/api/v1/me", get(auth_routes::me).patch(auth_routes::update_me))
        .route("/api/v1/me/presence", patch(auth_routes::update_presence))
        .route(
            "/api/v1/users",
            get(users::list_users).post(users::create_user),
        )
        .route("/api/v1/users/:id", patch(users::update_user))
        .route(
            "/api/v1/libraries",
            get(libraries::list_libraries).post(libraries::create_library),
        )
        .route(
            "/api/v1/libraries/:library_id",
            patch(libraries::update_library).delete(libraries::delete_library),
        )
        .route(
            "/api/v1/libraries/:library_id/members",
            get(members::list_members),
        )
        .route(
            "/api/v1/libraries/:library_id/members/:user_id",
            post(members::upsert_member).delete(members::remove_member),
        )
        .route(
            "/api/v1/libraries/:library_id/assets",
            get(assets::list_assets),
        )
        .route(
            "/api/v1/libraries/:library_id/activity",
            get(activity::list_activity),
        )
        .route("/api/v1/activity", get(activity::list_server_activity))
        .route(
            "/api/v1/storage-roots",
            get(storage_roots::list_storage_roots).post(storage_roots::create_storage_root),
        )
        .route(
            "/api/v1/storage-roots/:id",
            get(storage_roots::get_storage_root)
                .patch(storage_roots::update_storage_root)
                .delete(storage_roots::delete_storage_root),
        )
        .with_state(state)
}
