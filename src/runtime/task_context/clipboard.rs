//! Clipboard management methods for `TaskContext`.

use anyhow::Result;

use crate::runtime::task_context::TaskContext;

/// Compute the new clipboard content when appending text.
///
/// Pure function: given the current clipboard content, text to append,
/// and an optional separator, returns the combined string.
/// - If current is empty, returns just `text`.
/// - Otherwise returns `current + separator + text` (separator defaults to "").
#[must_use]
pub(crate) fn compute_appended_clipboard(
    current: &str,
    text: &str,
    separator: Option<&str>,
) -> String {
    if current.is_empty() {
        text.to_string()
    } else {
        format!("{}{}{}", current, separator.unwrap_or(""), text)
    }
}

impl TaskContext {
    pub fn read_clipboard(&self) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .ok_or_else(|| anyhow::anyhow!("Clipboard empty or session not found"))
    }

    pub fn write_clipboard(&self, text: &str) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone()).set(text);
        Ok(())
    }

    pub fn clear_clipboard(&self) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone()).set("");
        Ok(())
    }

    pub fn has_clipboard_content(&self) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        let has_content = crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .is_some_and(|s| !s.is_empty());
        Ok(has_content)
    }

    pub fn append_clipboard(&self, text: &str, separator: Option<&str>) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        let current = crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .unwrap_or_default();
        let new_content = compute_appended_clipboard(&current, text, separator);
        crate::state::ClipboardState::new(self.session_id.clone()).set(&new_content);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::compute_appended_clipboard;

    #[test]
    fn test_append_to_empty_clipboard() {
        let result = compute_appended_clipboard("", "hello", None);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_append_to_empty_with_separator() {
        let result = compute_appended_clipboard("", "hello", Some(", "));
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_append_to_non_empty_no_separator() {
        let result = compute_appended_clipboard("existing", "text", None);
        assert_eq!(result, "existingtext");
    }

    #[test]
    fn test_append_to_non_empty_with_separator() {
        let result = compute_appended_clipboard("existing", "text", Some(", "));
        assert_eq!(result, "existing, text");
    }

    #[test]
    fn test_append_to_empty_with_empty_text() {
        let result = compute_appended_clipboard("", "", None);
        assert_eq!(result, "");
    }

    #[test]
    fn test_append_to_non_empty_with_empty_text() {
        let result = compute_appended_clipboard("existing", "", Some(", "));
        assert_eq!(result, "existing, ");
    }

    #[test]
    fn test_append_multiple_times_with_separator() {
        let step1 = compute_appended_clipboard("", "a", Some(", "));
        assert_eq!(step1, "a");
        let step2 = compute_appended_clipboard(&step1, "b", Some(", "));
        assert_eq!(step2, "a, b");
        let step3 = compute_appended_clipboard(&step2, "c", Some(", "));
        assert_eq!(step3, "a, b, c");
    }

    #[test]
    fn test_append_with_newline_separator() {
        let result = compute_appended_clipboard("line1", "line2", Some("\n"));
        assert_eq!(result, "line1\nline2");
    }

    #[test]
    fn test_append_unicode_text() {
        let result = compute_appended_clipboard("Hello", "世界", Some(" "));
        assert_eq!(result, "Hello 世界");
    }

    #[test]
    fn test_append_empty_separator_string() {
        let result = compute_appended_clipboard("a", "b", Some(""));
        assert_eq!(result, "ab");
    }
}
