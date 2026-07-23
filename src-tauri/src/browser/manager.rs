use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use chrono::Utc;
use tauri::webview::{NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, Emitter as _, LogicalPosition, LogicalSize, Manager as _, WebviewUrl};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::browser::policy::{browser_url, platform_from_url};
use crate::domain::browser::{BrowserBounds, BrowserLoadState, BrowserTab};
use crate::error::{AppError, AppResult};

const MAX_BROWSER_TABS: usize = 5;

#[derive(Debug, Default)]
struct BrowserState {
    tabs: HashMap<Uuid, BrowserTab>,
    active: Option<Uuid>,
    bounds: Option<BrowserBounds>,
    cancellations: HashMap<Uuid, CancellationToken>,
}

#[derive(Debug, Clone, Default)]
pub struct BrowserManager {
    inner: Arc<Mutex<BrowserState>>,
}

impl BrowserManager {
    pub fn tabs(&self) -> Vec<BrowserTab> {
        let mut tabs = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tabs
            .values()
            .cloned()
            .collect::<Vec<_>>();
        tabs.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        tabs
    }

    pub fn tab(&self, id: Uuid) -> AppResult<BrowserTab> {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tabs
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("browser tab {id}")))
    }

    pub fn create_tab(
        &self,
        app: &AppHandle,
        initial_url: &str,
        bounds: BrowserBounds,
    ) -> AppResult<BrowserTab> {
        let url = browser_url(initial_url)?;
        let window = app
            .get_window("main")
            .ok_or_else(|| AppError::NotFound("main window".to_owned()))?;
        {
            let state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if state.tabs.len() >= MAX_BROWSER_TABS {
                return Err(AppError::Validation(format!(
                    "Tagline supports at most {MAX_BROWSER_TABS} browser tabs"
                )));
            }
        }

        self.hide_all(app)?;
        let id = Uuid::new_v4();
        let label = format!("browser-{id}");
        let tab = BrowserTab {
            id,
            webview_label: label.clone(),
            current_url: url.to_string(),
            title: "New tab".to_owned(),
            load_state: BrowserLoadState::Loading,
            platform: platform_from_url(&url),
            active: true,
            created_at: Utc::now().to_rfc3339(),
        };

        let navigation_manager = self.clone();
        let navigation_app = app.clone();
        let title_manager = self.clone();
        let title_app = app.clone();
        let load_manager = self.clone();
        let load_app = app.clone();
        let new_window_app = app.clone();
        let builder = WebviewBuilder::new(&label, WebviewUrl::External(url))
            .on_navigation(|candidate| browser_url(candidate.as_str()).is_ok())
            .on_document_title_changed(move |_webview, title| {
                title_manager.update_title(id, title);
                title_manager.emit_tab(&title_app, id);
            })
            .on_page_load(move |_webview, payload| {
                let load_state = match payload.event() {
                    PageLoadEvent::Started => BrowserLoadState::Loading,
                    PageLoadEvent::Finished => BrowserLoadState::Loaded,
                };
                load_manager.update_navigation(id, payload.url().as_str(), load_state);
                load_manager.emit_tab(&load_app, id);
            })
            .on_new_window(move |requested_url, _features| {
                let _ = new_window_app.emit_to(
                    "main",
                    "browser://new-window-requested",
                    serde_json::json!({"url": requested_url.as_str()}),
                );
                NewWindowResponse::Deny
            });

        window
            .add_child(
                builder,
                LogicalPosition::new(bounds.x, bounds.y),
                LogicalSize::new(bounds.width, bounds.height),
            )
            .map_err(|error| AppError::Internal(format!("browser engine failed: {error}")))?;

        {
            let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            for existing in state.tabs.values_mut() {
                existing.active = false;
            }
            state.bounds = Some(bounds);
            state.active = Some(id);
            state.tabs.insert(id, tab.clone());
        }
        navigation_manager.emit_tab(&navigation_app, id);
        Ok(tab)
    }

    pub fn activate(&self, app: &AppHandle, id: Uuid) -> AppResult<BrowserTab> {
        self.hide_all(app)?;
        let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let label = state
            .tabs
            .get(&id)
            .map(|tab| tab.webview_label.clone())
            .ok_or_else(|| AppError::NotFound(format!("browser tab {id}")))?;
        let webview = app
            .get_webview(&label)
            .ok_or_else(|| AppError::NotFound(format!("browser surface {label}")))?;
        if let Some(bounds) = state.bounds {
            webview
                .set_position(LogicalPosition::new(bounds.x, bounds.y))
                .and_then(|_| webview.set_size(LogicalSize::new(bounds.width, bounds.height)))
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        webview
            .show()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        for tab in state.tabs.values_mut() {
            tab.active = tab.id == id;
        }
        state.active = Some(id);
        state
            .tabs
            .get(&id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("browser tab {id}")))
    }

    pub fn set_bounds(&self, app: &AppHandle, bounds: BrowserBounds) -> AppResult<()> {
        let active = {
            let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            state.bounds = Some(bounds);
            state.active.and_then(|id| state.tabs.get(&id).cloned())
        };
        if let Some(tab) = active {
            let webview = app
                .get_webview(&tab.webview_label)
                .ok_or_else(|| AppError::NotFound("active browser surface".to_owned()))?;
            webview
                .set_position(LogicalPosition::new(bounds.x, bounds.y))
                .and_then(|_| webview.set_size(LogicalSize::new(bounds.width, bounds.height)))
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        Ok(())
    }

    pub fn navigate(&self, app: &AppHandle, id: Uuid, value: &str) -> AppResult<BrowserTab> {
        let url = browser_url(value)?;
        let tab = self.tab(id)?;
        app.get_webview(&tab.webview_label)
            .ok_or_else(|| AppError::NotFound("browser surface".to_owned()))?
            .navigate(url.clone())
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.update_navigation(id, url.as_str(), BrowserLoadState::Loading);
        self.tab(id)
    }

    pub fn reload(&self, app: &AppHandle, id: Uuid) -> AppResult<()> {
        let tab = self.tab(id)?;
        app.get_webview(&tab.webview_label)
            .ok_or_else(|| AppError::NotFound("browser surface".to_owned()))?
            .reload()
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    pub fn history(&self, app: &AppHandle, id: Uuid, delta: i8) -> AppResult<()> {
        let tab = self.tab(id)?;
        let script = if delta < 0 {
            "window.history.back()"
        } else {
            "window.history.forward()"
        };
        app.get_webview(&tab.webview_label)
            .ok_or_else(|| AppError::NotFound("browser surface".to_owned()))?
            .eval(script)
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    pub fn hide_all(&self, app: &AppHandle) -> AppResult<()> {
        for tab in self.tabs() {
            if let Some(webview) = app.get_webview(&tab.webview_label) {
                webview
                    .hide()
                    .map_err(|error| AppError::Internal(error.to_string()))?;
            }
        }
        let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        for tab in state.tabs.values_mut() {
            tab.active = false;
        }
        state.active = None;
        Ok(())
    }

    pub fn close(&self, app: &AppHandle, id: Uuid) -> AppResult<()> {
        let tab = self.tab(id)?;
        if let Some(webview) = app.get_webview(&tab.webview_label) {
            webview
                .close()
                .map_err(|error| AppError::Internal(error.to_string()))?;
        }
        let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        state.tabs.remove(&id);
        if state.active == Some(id) {
            state.active = None;
        }
        Ok(())
    }

    pub fn clear_data(&self, app: &AppHandle) -> AppResult<()> {
        let webview = self
            .tabs()
            .into_iter()
            .find_map(|tab| app.get_webview(&tab.webview_label))
            .or_else(|| app.get_webview("main"))
            .ok_or_else(|| AppError::NotFound("browser data store".to_owned()))?;
        webview
            .clear_all_browsing_data()
            .map_err(|error| AppError::Internal(error.to_string()))?;
        Ok(())
    }

    pub fn webview_label(&self, id: Uuid) -> AppResult<String> {
        Ok(self.tab(id)?.webview_label)
    }

    pub fn register_run(&self, run_id: Uuid) -> CancellationToken {
        let token = CancellationToken::new();
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cancellations
            .insert(run_id, token.clone());
        token
    }

    pub fn cancel_run(&self, run_id: Uuid) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cancellations
            .get(&run_id)
            .map(|token| {
                token.cancel();
                true
            })
            .unwrap_or(false)
    }

    pub fn finish_run(&self, run_id: Uuid) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .cancellations
            .remove(&run_id);
    }

    fn update_title(&self, id: Uuid, title: String) {
        if let Some(tab) = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tabs
            .get_mut(&id)
        {
            tab.title = title.chars().take(160).collect();
        }
    }

    fn update_navigation(&self, id: Uuid, value: &str, load_state: BrowserLoadState) {
        let Ok(url) = browser_url(value) else {
            return;
        };
        if let Some(tab) = self
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tabs
            .get_mut(&id)
        {
            tab.current_url = url.to_string();
            tab.platform = platform_from_url(&url);
            tab.load_state = load_state;
        }
    }

    fn emit_tab(&self, app: &AppHandle, id: Uuid) {
        if let Ok(tab) = self.tab(id) {
            let _ = app.emit_to("main", "browser://tab-updated", tab);
        }
    }
}
