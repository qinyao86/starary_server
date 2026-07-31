mod access;
mod activity;
mod assets;
mod auth_routes;
mod avatars;
mod backups;
mod health;
mod initialization;
mod libraries;
mod library_structure;
mod library_transfer;
mod members;
mod preferences;
mod presets;
mod runtime;
mod setup;
mod storage_connections;
mod storage_roots;
mod tasks;
pub(crate) mod users;

use crate::state::AppState;
use axum::{
    extract::DefaultBodyLimit,
    handler::Handler,
    response::Redirect,
    routing::{get, patch, post, put},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

pub fn router(state: AppState) -> Router {
    let admin_assets_dir = state.config.resolved_admin_assets_dir();
    let admin_index = admin_assets_dir.join("index.html");
    let admin_service = ServeDir::new(admin_assets_dir).fallback(ServeFile::new(admin_index));

    Router::new()
        .route("/", get(|| async { Redirect::temporary("/admin/") }))
        .route("/admin", get(|| async { Redirect::permanent("/admin/") }))
        .nest_service("/admin/", admin_service)
        .route("/health", get(health::health))
        .route("/api/v1/server/info", get(health::server_info))
        .route(
            "/api/v1/server/runtime",
            get(runtime::settings).patch(runtime::update_settings),
        )
        .route("/api/v1/server/shutdown", post(runtime::shutdown))
        .route(
            "/api/v1/server/desktop/identity",
            get(runtime::desktop_identity),
        )
        .route(
            "/api/v1/server/desktop/shutdown",
            post(runtime::desktop_shutdown),
        )
        .route(
            "/api/v1/backups",
            get(backups::overview).post(backups::create),
        )
        .route("/api/v1/backups/settings", patch(backups::update_settings))
        .route("/api/v1/backups/restore", post(backups::restore))
        .route(
            "/api/v1/backups/restore-file",
            post(backups::restore_file).layer(DefaultBodyLimit::max(2usize * 1024 * 1024 * 1024)),
        )
        .route(
            "/api/v1/server/initialize",
            post(initialization::initialize),
        )
        .route(
            "/api/v1/backups/:backup_id",
            get(backups::download).delete(backups::delete),
        )
        .route("/api/v1/setup/status", get(setup::setup_status))
        .route("/api/v1/setup/owner", post(setup::create_owner))
        .route("/api/v1/auth/login", post(auth_routes::login))
        .route(
            "/api/v1/auth/browser-handoff",
            post(auth_routes::create_browser_handoff),
        )
        .route(
            "/api/v1/auth/browser-handoff/redeem",
            post(auth_routes::redeem_browser_handoff),
        )
        .route(
            "/api/v1/me",
            get(auth_routes::me).patch(auth_routes::update_me),
        )
        .route("/api/v1/me/password", post(auth_routes::change_my_password))
        .route("/api/v1/me/presence", patch(auth_routes::update_presence))
        .route("/api/v1/me/libraries", get(libraries::list_my_libraries))
        .route(
            "/api/v1/me/library-status",
            get(libraries::list_my_library_statuses),
        )
        .route("/api/v1/avatars/system", get(avatars::list_system_avatars))
        .route(
            "/api/v1/avatars/system/:key",
            get(avatars::read_system_avatar),
        )
        .route(
            "/api/v1/avatars/users/:user_id",
            get(avatars::read_user_avatar),
        )
        .route(
            "/api/v1/users",
            get(users::list_users).post(users::create_user),
        )
        .route(
            "/api/v1/users/:id",
            patch(users::update_user).delete(users::delete_user),
        )
        .route(
            "/api/v1/users/:id/avatar",
            patch(users::update_user_avatar)
                .put(avatars::upload_user_avatar)
                .delete(avatars::delete_user_avatar)
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        .route(
            "/api/v1/libraries",
            get(libraries::list_libraries).post(libraries::create_library),
        )
        .route(
            "/api/v1/library-status",
            get(libraries::list_library_statuses),
        )
        .route(
            "/api/v1/libraries/:library_id",
            patch(libraries::update_library).delete(libraries::delete_library),
        )
        .route(
            "/api/v1/libraries/:library_id/icon",
            put(libraries::upload_library_icon).delete(libraries::clear_library_icon),
        )
        .route(
            "/api/v1/libraries/:library_id/icon/from-asset",
            put(libraries::set_library_icon_from_asset),
        )
        .route(
            "/api/v1/libraries/:library_id/join",
            post(libraries::join_library),
        )
        .route(
            "/api/v1/libraries/:library_id/enabled",
            patch(libraries::update_library_enabled),
        )
        .route(
            "/api/v1/libraries/:library_id/storage-binding",
            post(libraries::assign_library_storage),
        )
        .route(
            "/api/v1/libraries/:library_id/members",
            get(members::list_members),
        )
        .route(
            "/api/v1/libraries/:library_id/contributors",
            get(members::list_contributors),
        )
        .route(
            "/api/v1/libraries/:library_id/members/:user_id",
            post(members::upsert_member).delete(members::remove_member),
        )
        .route(
            "/api/v1/libraries/:library_id/assets",
            get(assets::list_assets)
                .post(assets::import_assets.layer(DefaultBodyLimit::max(384 * 1024 * 1024)))
                .delete(assets::delete_assets_permanently),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/query",
            post(assets::query_assets),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/query/ids",
            post(assets::query_asset_ids),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/mutation-access/check",
            post(assets::check_asset_mutation_access),
        )
        .route(
            "/api/v1/libraries/:library_id/transfer/asset",
            post(library_transfer::transfer_asset),
        )
        .route(
            "/api/v1/libraries/:library_id/transfer/folder",
            post(library_transfer::transfer_folder),
        )
        .route(
            "/api/v1/libraries/:library_id/transfer/export/assets/:asset_id",
            get(library_transfer::export_asset),
        )
        .route(
            "/api/v1/libraries/:library_id/transfer/export/folders/:folder_id",
            get(library_transfer::export_folder),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/rating",
            patch(assets::update_assets_rating),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/starred",
            patch(assets::update_assets_starred),
        )
        .route(
            "/api/v1/libraries/:library_id/preferences",
            get(preferences::get_library_preferences),
        )
        .route(
            "/api/v1/libraries/:library_id/preferences/quick-access",
            put(preferences::update_quick_access),
        )
        .route(
            "/api/v1/libraries/:library_id/preferences/filter-presets",
            get(preferences::list_filter_presets)
                .post(preferences::create_filter_preset)
                .delete(preferences::clear_filter_presets),
        )
        .route(
            "/api/v1/libraries/:library_id/preferences/filter-presets/reorder",
            put(preferences::reorder_filter_presets),
        )
        .route(
            "/api/v1/libraries/:library_id/preferences/filter-presets/:preset_id",
            patch(preferences::update_filter_preset).delete(preferences::delete_filter_preset),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/viewer",
            patch(assets::update_assets_viewer),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/import-mode",
            patch(assets::convert_assets_import_mode),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/folders",
            patch(assets::set_asset_folders),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/tags",
            patch(assets::set_asset_tags),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/trash",
            post(assets::trash_assets),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/restore",
            post(assets::restore_assets),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/duplicates/merge",
            post(assets::merge_duplicate_assets),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/:asset_id/derived",
            patch(assets::update_asset_derived_files),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/:asset_id/text",
            get(assets::read_asset_text).patch(assets::update_asset_text),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/:asset_id/sequence/frame-numbers",
            patch(assets::update_image_sequence_frame_numbers),
        )
        .route(
            "/api/v1/libraries/:library_id/assets/:asset_id",
            patch(assets::update_asset),
        )
        .route(
            "/api/v1/libraries/:library_id/storage-roots/:storage_root_id/files/*relative_path",
            get(assets::read_library_storage_file),
        )
        .route(
            "/api/v1/libraries/:library_id/folders",
            get(library_structure::list_folders).post(library_structure::create_folder),
        )
        .route(
            "/api/v1/libraries/:library_id/folders/reorder",
            post(library_structure::reorder_folders),
        )
        .route(
            "/api/v1/libraries/:library_id/folders/import-plan",
            post(library_structure::create_folder_import_plan),
        )
        .route(
            "/api/v1/libraries/:library_id/folders/:folder_id",
            patch(library_structure::update_folder).delete(library_structure::delete_folder),
        )
        .route(
            "/api/v1/libraries/:library_id/tag-groups",
            get(library_structure::list_tag_groups).post(library_structure::create_tag_group),
        )
        .route(
            "/api/v1/libraries/:library_id/tag-groups/:group_id",
            patch(library_structure::update_tag_group).delete(library_structure::delete_tag_group),
        )
        .route(
            "/api/v1/libraries/:library_id/tags",
            get(library_structure::list_tags).post(library_structure::create_tag),
        )
        .route(
            "/api/v1/libraries/:library_id/tags/move",
            post(library_structure::move_tags),
        )
        .route(
            "/api/v1/libraries/:library_id/tags/:tag_id",
            patch(library_structure::update_tag).delete(library_structure::delete_tag),
        )
        .route(
            "/api/v1/libraries/:library_id/presets/:preset_type",
            get(presets::list_presets)
                .post(presets::create_preset)
                .delete(presets::clear_presets),
        )
        .route(
            "/api/v1/libraries/:library_id/presets/:preset_type/reorder",
            post(presets::reorder_presets),
        )
        .route(
            "/api/v1/libraries/:library_id/presets/:preset_type/:preset_id",
            patch(presets::update_preset).delete(presets::delete_preset),
        )
        .route(
            "/api/v1/libraries/:library_id/presets/:preset_type/:preset_id/count",
            patch(presets::update_preset_count),
        )
        .route(
            "/api/v1/libraries/:library_id/activity",
            get(activity::list_activity),
        )
        .route("/api/v1/activity", get(activity::list_server_activity))
        .route("/api/v1/tasks", get(tasks::list_tasks))
        .route(
            "/api/v1/tasks/:task_id",
            put(tasks::report_task).delete(tasks::delete_task),
        )
        .route(
            "/api/v1/storage-connections",
            get(storage_connections::list_storage_connections)
                .post(storage_connections::create_storage_connection),
        )
        .route(
            "/api/v1/storage-connections/:id",
            patch(storage_connections::update_storage_connection)
                .delete(storage_connections::delete_storage_connection),
        )
        .route(
            "/api/v1/storage-connections/:id/default",
            patch(storage_connections::set_default_storage_connection),
        )
        .route(
            "/api/v1/storage-connections/:id/migrate",
            post(storage_connections::migrate_storage_connection),
        )
        .route(
            "/api/v1/storage-roots",
            get(storage_roots::list_storage_roots),
        )
        .route(
            "/api/v1/storage-roots/:id",
            get(storage_roots::get_storage_root),
        )
        .with_state(state)
}
