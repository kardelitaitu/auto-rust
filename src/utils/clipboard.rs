use anyhow::Result;
use chromiumoxide::Page;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde_json::Value;

static SESSION_CLIPBOARD: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

#[derive(Clone, Debug)]
pub struct ClipboardState {
    session_id: String,
}

impl ClipboardState {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn set(&self, text: impl Into<String>) {
        set_clipboard(&self.session_id, text);
    }

    pub fn get(&self) -> Option<String> {
        get_clipboard(&self.session_id)
    }

    pub fn clear(&self) {
        clear_clipboard(&self.session_id);
    }

    pub async fn copy(&self, page: &Page) -> Result<String> {
        copy(&self.session_id, page).await
    }

    pub async fn cut(&self, page: &Page) -> Result<String> {
        cut(&self.session_id, page).await
    }

    pub async fn paste(&self, page: &Page) -> Result<String> {
        paste_from_clipboard(&self.session_id, page).await
    }
}

pub fn set_session_clipboard(session_id: &str, text: impl Into<String>) {
    SESSION_CLIPBOARD.insert(session_id.to_string(), text.into());
}

pub fn get_session_clipboard(session_id: &str) -> Option<String> {
    SESSION_CLIPBOARD.get(session_id).map(|v| v.value().clone())
}

pub fn clear_session_clipboard(session_id: &str) {
    SESSION_CLIPBOARD.remove(session_id);
}

pub fn set_clipboard(session_id: &str, text: impl Into<String>) {
    set_session_clipboard(session_id, text);
}

pub fn get_clipboard(session_id: &str) -> Option<String> {
    get_session_clipboard(session_id)
}

pub fn clear_clipboard(session_id: &str) {
    clear_session_clipboard(session_id);
}

pub async fn copy_selection(session_id: &str, page: &Page) -> Result<String> {
    let text = read_selection_text(page).await?;
    set_session_clipboard(session_id, text.clone());
    Ok(text)
}

pub async fn copy(session_id: &str, page: &Page) -> Result<String> {
    copy_selection(session_id, page).await
}

pub async fn cut_selection(session_id: &str, page: &Page) -> Result<String> {
    let text = read_selection_text(page).await?;
    if text.is_empty() {
        return Ok(text);
    }

    replace_selection(page, "").await?;
    set_session_clipboard(session_id, text.clone());
    Ok(text)
}

pub async fn cut(session_id: &str, page: &Page) -> Result<String> {
    cut_selection(session_id, page).await
}

pub async fn paste(session_id: &str, page: &Page) -> Result<String> {
    let text = get_session_clipboard(session_id).unwrap_or_default();
    if text.is_empty() {
        return Ok(text);
    }

    insert_text(page, &text).await?;
    Ok(text)
}

pub async fn paste_from_clipboard(session_id: &str, page: &Page) -> Result<String> {
    paste(session_id, page).await
}

async fn read_selection_text(page: &Page) -> Result<String> {
    let result = page
        .evaluate(
            r#"
        (function() {
            const el = document.activeElement;
            if (!el) return "";

            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
                const start = el.selectionStart ?? 0;
                const end = el.selectionEnd ?? 0;
                return el.value.slice(start, end);
            }

            if (el.isContentEditable) {
                return window.getSelection().toString();
            }

            return "";
        })();
        "#,
        )
        .await?;

    let value = result.value().cloned().unwrap_or(Value::Null);
    Ok(value.as_str().unwrap_or("").to_string())
}

async fn replace_selection(page: &Page, replacement: &str) -> Result<()> {
    let replacement_json = serde_json::to_string(replacement)?;
    page.evaluate(format!(
        r#"
        (function() {{
            const el = document.activeElement;
            if (!el) return;

            const text = {replacement_json};
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                const start = el.selectionStart ?? 0;
                const end = el.selectionEnd ?? 0;
                el.value = el.value.slice(0, start) + text + el.value.slice(end);
                const caret = start + text.length;
                el.selectionStart = caret;
                el.selectionEnd = caret;
                el.dispatchEvent(new InputEvent('input', {{ bubbles: true, data: text }}));
            }} else if (el.isContentEditable) {{
                document.execCommand('insertText', false, text);
            }}
        }})();
        "#
    ))
    .await?;
    Ok(())
}

async fn insert_text(page: &Page, text: &str) -> Result<()> {
    let text_json = serde_json::to_string(text)?;
    page.evaluate(format!(
        r#"
        (function() {{
            const el = document.activeElement;
            if (!el) return;

            const text = {text_json};
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                const start = el.selectionStart ?? el.value.length;
                const end = el.selectionEnd ?? el.value.length;
                el.value = el.value.slice(0, start) + text + el.value.slice(end);
                const caret = start + text.length;
                el.selectionStart = caret;
                el.selectionEnd = caret;
                el.dispatchEvent(new InputEvent('input', {{ bubbles: true, data: text }}));
            }} else if (el.isContentEditable) {{
                document.execCommand('insertText', false, text);
            }}
        }})();
        "#
    ))
    .await?;
    Ok(())
}
