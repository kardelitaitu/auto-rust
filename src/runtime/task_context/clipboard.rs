//! Clipboard management methods for `TaskContext`.

use anyhow::Result;

use crate::runtime::task_context::TaskContext;

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
        let new_content = if current.is_empty() {
            text.to_string()
        } else {
            format!("{}{}{}", current, separator.unwrap_or(""), text)
        };
        crate::state::ClipboardState::new(self.session_id.clone()).set(&new_content);
        Ok(())
    }
}
