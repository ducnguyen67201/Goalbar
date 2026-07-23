#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod adapters;
pub mod app_state;
pub mod commands;
pub mod conductor;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod logging;
pub mod secrets;
pub mod services;
pub mod validation;

use tauri::Manager as _;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init();
    let result = tauri::Builder::default()
        .setup(|app| {
            let default_dir = app.path().app_data_dir()?;
            let data_dir = config::resolve_data_dir(default_dir)
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            let database_path = data_dir.join("tagline.sqlite");
            let state = tauri::async_runtime::block_on(app_state::AppState::open(&database_path))
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            services::scheduler::start(state.clone());
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::get_bootstrap_state,
            commands::agents::detect_agents,
            commands::agents::run_agent_task,
            commands::agents::cancel_job,
            commands::onboarding::save_founder_profile,
            commands::onboarding::save_voice_profile,
            commands::onboarding::generate_icp_hypotheses,
            commands::onboarding::list_icp_hypotheses,
            commands::onboarding::accept_icp_hypothesis,
            commands::content::generate_content_variants,
            commands::content::approve_variant,
            commands::content::publish_variant,
            commands::platforms::list_platform_statuses,
            commands::platforms::begin_platform_oauth,
            commands::platforms::get_oauth_status,
            commands::platforms::complete_platform_oauth,
            commands::platforms::disconnect_platform,
            commands::platforms::sync_platform_now,
            commands::inbox::list_conversations,
            commands::inbox::draft_reply,
            commands::inbox::approve_reply,
            commands::inbox::send_reply,
            commands::growth::get_growth_overview,
            commands::growth::generate_weekly_review,
            commands::growth::accept_learning,
            commands::settings::check_keyring,
            commands::settings::open_remote_url,
            commands::settings::export_local_data,
            commands::settings::backup_local_database,
            commands::settings::factory_reset_local_data,
        ])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        panic!("failed to run Tagline: {error}");
    }
}
