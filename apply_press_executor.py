#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Apply Press action changes to executor.rs."""

import sys
import os

# Force UTF-8 output
sys.stdout.reconfigure(encoding='utf-8')

with open('src/task/dsl/executor.rs', 'r', encoding='utf-8') as f:
    content = f.read()

changes = 0

# 1. Add Press dispatch arm after DoubleClick
old_dispatch = "            Action::DoubleClick { selector } => self.execute_double_click(selector).await,\n            Action::Parallel {"
new_dispatch = "            Action::DoubleClick { selector } => self.execute_double_click(selector).await,\n            Action::Press { key, modifiers } => self.execute_press(key, modifiers.as_deref()).await,\n            Action::Parallel {"
if old_dispatch in content:
    content = content.replace(old_dispatch, new_dispatch, 1)
    changes += 1
    print(f"+ Dispatch arm added (change {changes})")
else:
    print("! Dispatch arm: pattern not found")
    idx = content.find("Action::DoubleClick { selector } => self.execute_double_click(selector).await,")
    if idx >= 0:
        print(f"  Found DoubleClick at {idx}")
        print(repr(content[idx:idx+100]))

# 2. Add execute_press handler after execute_double_click
old_handler = "        Ok(())\n    }\n\n    /// Cached wrapper for checking element existence."
new_handler = """        Ok(())\n    }\n\n    /// Press a key, optionally with modifier keys.\n    async fn execute_press(&mut self, key: &str, modifiers: Option<&[String]>) -> Result<()> {\n        let resolved_key = self.substitute_variables(key);\n        let mut resolved_mods: Vec<String> = Vec::new();\n        if let Some(mods) = modifiers {\n            for mod_key in mods {\n                resolved_mods.push(self.substitute_variables(mod_key));\n            }\n        }\n\n        log::debug!(\n            \"Pressing key '{}' with modifiers {:?}\",\n            resolved_key,\n            resolved_mods\n        );\n\n        self.api\n            .press(\n                &resolved_key,\n                &resolved_mods.iter().map(String::as_str).collect::<Vec<_>>(),\n            )\n            .await?;\n        self.clear_cache();\n        Ok(())\n    }\n\n    /// Cached wrapper for checking element existence."""
if old_handler in content:
    content = content.replace(old_handler, new_handler, 1)
    changes += 1
    print(f"+ Handler method added (change {changes})")
else:
    print("! Handler: pattern not found")
    idx = content.find("Ok(())\n    }\n\n    /// Cached wrapper")
    if idx >= 0:
        print(f"  Found target at offset {idx}")
        print(repr(content[idx:idx+150]))

# 3. Add Press unit tests before the pipeline integration tests section
pipeline_header = "    // --- Parser -> Executor pipeline integration tests ---------------------\n    //\n    // These tests verify the full path: YAML/TOML string -> parse_task_*() ->\n    // TaskDefinition -> DslExecutor::execute() -> verify MockDslApi calls.\n    //\n    // NOTE: Use r## for YAML containing CSS selectors like \"#main\" (avoid \"#\n    // being interpreted as raw string delimiter).\n\n    #[tokio::test]\n    async fn test_pipeline_execute_yaml_full_flow() {"

# Try with the actual non-ASCII box-drawing characters
# Let's find the actual text at the pipeline section
idx_pipeline_header = content.find("Parser -> Executor pipeline integration tests")
if idx_pipeline_header >= 0:
    # Find the start of the line
    line_start = content.rfind('\n', 0, idx_pipeline_header) + 1
    # Find the end of the function signature
    func_start = content.find("async fn test_pipeline_execute_yaml_full_flow()", idx_pipeline_header)
    if func_start >= 0:
        # Find the line start of this function
        func_line_start = content.rfind('\n', 0, func_start) + 1
        # Get the text from the comment header to right before the function body
        header_text = content[line_start:func_line_start]
        # Read the next line too (the function definition with its opening brace)
        func_def_line_end = content.find('\n', func_line_start)
        func_def_line = content[func_line_start:func_def_line_end]
        
        print(f"Pipeline header section found at offset {line_start}:")
        print(repr(header_text))
        print(f"Function def line: {repr(func_def_line)}")
        
        # Now use this to construct the replacement
        full_old = content[line_start:func_def_line_end]
        
        new_tests = """    // --- Press action --------------------------------------------------------

    #[tokio::test]
    async fn test_execute_action_press_calls_api() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Press {
            key: "Enter".to_string(),
            modifiers: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Press should make exactly one API call");
        assert_eq!(
            calls[0],
            MockCall::Press {
                key: "Enter".to_string(),
                modifiers: vec![],
            },
            "Press should call api.press with the correct key"
        );
    }

    #[tokio::test]
    async fn test_execute_action_press_with_modifiers() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.execute_action(&Action::Press {
            key: "a".to_string(),
            modifiers: Some(vec!["Control".to_string(), "Shift".to_string()]),
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1, "Press with modifiers should make one call");
        assert_eq!(
            calls[0],
            MockCall::Press {
                key: "a".to_string(),
                modifiers: vec!["Control".to_string(), "Shift".to_string()],
            },
            "Press should pass through modifiers"
        );
    }

    #[tokio::test]
    async fn test_execute_action_press_with_variable_substitution() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);
        exec.variables
            .insert("the_key".to_string(), "Delete".to_string());

        exec.execute_action(&Action::Press {
            key: "${the_key}".to_string(),
            modifiers: None,
        })
        .await
        .unwrap();

        let calls = mock.get_calls();
        assert_eq!(
            calls[0],
            MockCall::Press {
                key: "Delete".to_string(),
                modifiers: vec![],
            },
            "variables should be substituted before calling press"
        );
    }

    #[tokio::test]
    async fn test_execute_action_press_clears_cache() {
        let mock = MockDslApi::new();
        let mut exec = create_executor(&mock, vec![]);

        exec.selector_cache.insert(
            "#stale".to_string(),
            crate::task::dsl::cache::SelectorCacheEntry::new(true, true, None, 0),
        );
        assert!(
            exec.cache_size() > 0,
            "cache should have entries before press"
        );

        exec.execute_action(&Action::Press {
            key: "Escape".to_string(),
            modifiers: None,
        })
        .await
        .unwrap();

        assert_eq!(
            exec.cache_size(),
            0,
            "cache should be cleared after press"
        );
    }

    #[tokio::test]
    async fn test_execute_action_press_propagates_error() {
        let mock = MockDslApi::new();
        mock.set_fail_all(true);
        let mut exec = create_executor(&mock, vec![]);

        let result = exec
            .execute_action(&Action::Press {
                key: "Enter".to_string(),
                modifiers: None,
            })
            .await;

        assert!(result.is_err(), "press should propagate API error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("MockDslApi forced failure"),
            "error should come from mock"
        );
    }

"""
        content = content[:line_start] + new_tests + content[line_start:]
        changes += 1
        print(f"+ Press unit tests added (change {changes})")
    else:
        print("! Could not find function definition")
else:
    print("! Pipeline section not found via string search")
    # Try searching with non-ASCII chars
    for i, line in enumerate(content.split('\n')):
        if 'Parser' in line and 'Executor' in line and 'pipeline' in line:
            print(f"  Found at line {i+1}: {repr(line[:60])}")

with open('src/task/dsl/executor.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print(f"\nTotal changes applied: {changes}")
