//! Files and stored settings, on desktop and in the browser.
//!
//! Every artefact this app produces is UTF-8 text (SVG, OBJ, JSON), so the
//! interface takes `&str` rather than bytes — which also keeps the wasm path to
//! a single-string `Blob`.
//!
//! # Saving is synchronous, opening is not
//!
//! Saving blocks: `rfd`'s native dialog returns a path, and the browser's
//! download needs no answer at all. Opening cannot, because on the web the file
//! only arrives after the user has picked it, in a future the frame loop cannot
//! await. Rather than have two shapes, *both* targets deliver through an
//! [`Inbox`] the UI drains each frame — the native side simply fills it before
//! returning.

/// What happened to a save request.
///
/// Every variant is present on every target so callers can report on any of
/// them without `cfg`; each is only constructed where it applies, which is why
/// dead code is allowed rather than conditionally compiled away.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Saved {
    /// Written to this path (desktop only).
    To(String),
    /// Handed to the browser as a download (web only).
    Downloaded,
    /// The user dismissed the dialog.
    Cancelled,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save_text(suggested_name: &str, contents: &str) -> Result<Saved, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(suggested_name)
        .save_file()
    else {
        return Ok(Saved::Cancelled);
    };

    std::fs::write(&path, contents).map_err(|e| format!("writing {}: {e}", path.display()))?;
    Ok(Saved::To(path.display().to_string()))
}

/// Trigger a browser download.
///
/// There is no filesystem here, so this builds a `Blob`, points a synthetic
/// anchor at it, and clicks it. The object URL is revoked immediately —
/// otherwise it leaks for the lifetime of the tab.
///
/// Note this runs from Bevy's frame loop rather than directly inside the click
/// handler. Browsers permit programmatic downloads outside a user gesture
/// (unlike `window.open`), though a browser that blocks repeated automatic
/// downloads may throttle rapid successive saves.
#[cfg(target_arch = "wasm32")]
pub fn save_text(suggested_name: &str, contents: &str) -> Result<Saved, String> {
    use wasm_bindgen::JsCast;

    let describe = |what: &str, e: wasm_bindgen::JsValue| format!("{what}: {e:?}");

    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(contents));

    let blob =
        web_sys::Blob::new_with_str_sequence(&parts).map_err(|e| describe("creating blob", e))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| describe("creating object URL", e))?;

    let result = (|| {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| "no document".to_string())?;
        let anchor = document
            .create_element("a")
            .map_err(|e| describe("creating anchor", e))?
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .map_err(|_| "element is not an anchor".to_string())?;
        anchor.set_href(&url);
        anchor.set_download(suggested_name);
        anchor.click();
        Ok(Saved::Downloaded)
    })();

    // Revoke whether or not the click succeeded.
    let _ = web_sys::Url::revoke_object_url(&url);
    result
}

// ---------------------------------------------------------------------------
// Opening a file
// ---------------------------------------------------------------------------

use std::sync::{Arc, Mutex};

/// What came back from a file picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Opened {
    /// The user chose a file, and this is what was in it.
    File { name: String, text: String },
    /// The user dismissed the dialog.
    Cancelled,
}

/// Where an in-flight file pick delivers its result.
///
/// Cloneable and shared: the wasm side hands a clone to a spawned future, and
/// the UI keeps the original to drain. A `Mutex` rather than a `RefCell`
/// because Bevy requires its resources to be `Send + Sync` on every target.
#[derive(Clone, Default)]
pub struct Inbox(Arc<Mutex<Option<Result<Opened, String>>>>);

impl Inbox {
    /// Take the result of a finished pick, if one has arrived.
    pub fn take(&self) -> Option<Result<Opened, String>> {
        self.0.lock().ok()?.take()
    }

    fn put(&self, result: Result<Opened, String>) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(result);
        }
    }
}

/// Ask the user for a JSON file. The answer arrives through `inbox`.
#[cfg(not(target_arch = "wasm32"))]
pub fn open_json(inbox: &Inbox) {
    let picked = rfd::FileDialog::new()
        .add_filter("Settings", &["json"])
        .pick_file();

    inbox.put(match picked {
        None => Ok(Opened::Cancelled),
        Some(path) => std::fs::read_to_string(&path)
            .map(|text| Opened::File {
                name: path.display().to_string(),
                text,
            })
            .map_err(|e| format!("reading {}: {e}", path.display())),
    });
}

#[cfg(target_arch = "wasm32")]
pub fn open_json(inbox: &Inbox) {
    let inbox = inbox.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let picked = rfd::AsyncFileDialog::new()
            .add_filter("Settings", &["json"])
            .pick_file()
            .await;

        inbox.put(match picked {
            None => Ok(Opened::Cancelled),
            Some(handle) => {
                let name = handle.file_name();
                String::from_utf8(handle.read().await)
                    .map(|text| Opened::File { name, text })
                    .map_err(|e| format!("that file is not UTF-8 text: {e}"))
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Settings remembered between sessions
// ---------------------------------------------------------------------------

/// Filename under the config directory, and the `localStorage` key.
const REMEMBERED: &str = "settings.json";

#[cfg(not(target_arch = "wasm32"))]
fn config_path() -> Option<std::path::PathBuf> {
    // Qualifier/organisation/application, per the app ID proposed in §12.5.
    let dirs = directories::ProjectDirs::from("io.github", "aero530", "ellipsoid")?;
    Some(dirs.config_dir().join(REMEMBERED))
}

/// Where the remembered settings live, phrased for a person to read.
///
/// Worth saying out loud: settings that reappear by themselves are a mystery
/// until you know where they came from, and on desktop this is the only way to
/// find the file if you want it gone.
#[cfg(not(target_arch = "wasm32"))]
pub fn remembered_location() -> Option<String> {
    Some(config_path()?.display().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recall() -> Option<String> {
    std::fs::read_to_string(config_path()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn remember(text: &str) -> Result<(), String> {
    let path = config_path().ok_or("no config directory on this system")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("creating {}: {e}", parent.display()))?;
    }
    std::fs::write(&path, text).map_err(|e| format!("writing {}: {e}", path.display()))
}

#[cfg(target_arch = "wasm32")]
pub fn remembered_location() -> Option<String> {
    Some("this browser's local storage".into())
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    // `local_storage()` is `Err` when storage is blocked by policy, and `Ok(None)`
    // when the context has none; neither is worth distinguishing here.
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn recall() -> Option<String> {
    storage()?.get_item(REMEMBERED).ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn remember(text: &str) -> Result<(), String> {
    storage()
        .ok_or("this browser has no local storage available")?
        .set_item(REMEMBERED, text)
        .map_err(|e| format!("local storage refused the write: {e:?}"))
}
