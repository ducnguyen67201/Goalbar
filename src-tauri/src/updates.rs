use std::time::Duration;

use tauri::{AppHandle, Runtime, plugin::TauriPlugin};
use tauri_plugin_updater::{Config, UpdaterExt as _};

const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const UPDATER_PUBLIC_KEY: Option<&str> = option_env!("GOALBAR_UPDATER_PUBLIC_KEY");

pub fn plugin<R: Runtime>() -> TauriPlugin<R, Config> {
    let builder = tauri_plugin_updater::Builder::new();

    match configured_public_key() {
        Some(public_key) => builder.pubkey(public_key).build(),
        None => builder.build(),
    }
}

pub fn start(app: AppHandle) {
    if cfg!(debug_assertions) {
        tracing::debug!("automatic app updates are disabled in development builds");
        return;
    }

    if configured_public_key().is_none() {
        tracing::warn!(
            "automatic app updates are disabled because this build has no updater public key"
        );
        return;
    }

    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(error) = check_and_install(&app).await {
                tracing::warn!(error = %error, "automatic app update check failed");
            }

            tokio::time::sleep(UPDATE_CHECK_INTERVAL).await;
        }
    });
}

async fn check_and_install(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };

    tracing::info!(
        current_version = %update.current_version,
        next_version = %update.version,
        "installing automatic app update"
    );

    update
        .download_and_install(
            |_chunk_length, _content_length| {},
            || tracing::info!("automatic app update downloaded"),
        )
        .await?;

    tracing::info!(version = %update.version, "automatic app update installed; relaunching");
    app.restart()
}

fn configured_public_key() -> Option<&'static str> {
    normalize_public_key(UPDATER_PUBLIC_KEY)
}

fn normalize_public_key(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|key| !key.is_empty())
}

#[cfg(test)]
mod tests {
    use super::normalize_public_key;

    #[test]
    fn updater_key_must_contain_non_whitespace_content() {
        assert_eq!(normalize_public_key(None), None);
        assert_eq!(normalize_public_key(Some(" \n ")), None);
        assert_eq!(
            normalize_public_key(Some(" public-key\n")),
            Some("public-key")
        );
    }
}
