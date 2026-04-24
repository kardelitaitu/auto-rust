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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_state_creation() {
        let state = ClipboardState::new("session-123");
        assert_eq!(state.session_id(), "session-123");
    }

    #[test]
    fn test_clipboard_state_set_get() {
        let state = ClipboardState::new("session-test");
        state.set("test content");
        assert_eq!(state.get(), Some("test content".to_string()));
    }

    #[test]
    fn test_clipboard_state_clear() {
        let state = ClipboardState::new("session-clear");
        state.set("content");
        state.clear();
        assert_eq!(state.get(), None);
    }

    #[test]
    fn test_set_clipboard() {
        set_clipboard("session-1", "hello world");
        assert_eq!(get_clipboard("session-1"), Some("hello world".to_string()));
    }

    #[test]
    fn test_get_clipboard_empty() {
        assert_eq!(get_clipboard("nonexistent"), None);
    }

    #[test]
    fn test_clear_clipboard() {
        set_clipboard("session-2", "to be cleared");
        clear_clipboard("session-2");
        assert_eq!(get_clipboard("session-2"), None);
    }

    #[test]
    fn test_set_session_clipboard() {
        set_session_clipboard("session-3", "session content");
        assert_eq!(get_session_clipboard("session-3"), Some("session content".to_string()));
    }

    #[test]
    fn test_get_session_clipboard_empty() {
        assert_eq!(get_session_clipboard("empty-session"), None);
    }

    #[test]
    fn test_clear_session_clipboard() {
        set_session_clipboard("session-4", "data");
        clear_session_clipboard("session-4");
        assert_eq!(get_session_clipboard("session-4"), None);
    }

    #[test]
    fn test_clipboard_state_clone() {
        let state1 = ClipboardState::new("session-clone");
        state1.set("original");
        let state2 = state1.clone();
        assert_eq!(state2.get(), Some("original".to_string()));
    }

    #[test]
    fn test_clipboard_state_multiple_sessions() {
        let state1 = ClipboardState::new("session-a");
        let state2 = ClipboardState::new("session-b");
        state1.set("content a");
        state2.set("content b");
        assert_eq!(state1.get(), Some("content a".to_string()));
        assert_eq!(state2.get(), Some("content b".to_string()));
    }

    #[test]
    fn test_clipboard_state_overwrite() {
        let state = ClipboardState::new("session-overwrite");
        state.set("first");
        state.set("second");
        assert_eq!(state.get(), Some("second".to_string()));
    }

    #[test]
    fn test_clipboard_state_empty_string() {
        let state = ClipboardState::new("session-empty");
        state.set("");
        assert_eq!(state.get(), Some("".to_string()));
    }

    #[test]
    fn test_clipboard_state_special_chars() {
        let state = ClipboardState::new("session-special");
        state.set("hello\nworld\t!");
        assert_eq!(state.get(), Some("hello\nworld\t!".to_string()));
    }

    #[test]
    fn test_clipboard_state_unicode() {
        let state = ClipboardState::new("session-unicode");
        state.set("🎉 test 🚀");
        assert_eq!(state.get(), Some("🎉 test 🚀".to_string()));
    }

    #[test]
    fn test_clipboard_state_long_text() {
        let state = ClipboardState::new("session-long");
        let long_text = "a".repeat(10000);
        state.set(&long_text);
        assert_eq!(state.get(), Some(long_text));
    }
}
