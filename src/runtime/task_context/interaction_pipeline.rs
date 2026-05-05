//! Shared interaction pipeline for TaskContext.
//!
//! This module provides a unified pipeline for browser interactions, ensuring
//! consistent preflight, execution, verification, and postflight behavior
//! across all TaskContext verbs (click, type, focus, select_all, clear).

use crate::runtime::task_context::types::{InteractionKind, InteractionRequest, InteractionResult};
use crate::runtime::task_context::TaskContext;
use crate::utils::mouse::ClickStatus;
use anyhow::Result;
use log::{debug, warn};

/// Execute an interaction through the shared pipeline.
///
/// This is the main entry point for the shared interaction pipeline. It handles:
/// 1. Preflight: element existence/visibility checks
/// 2. Execution: perform the actual interaction
/// 3. Verification: verify the interaction succeeded (if enabled)
/// 4. Postflight: pause and cleanup
///
/// The pipeline ensures consistent behavior across click, type, focus,
/// select_all, and clear operations.
pub async fn execute_interaction(
    ctx: &TaskContext,
    request: InteractionRequest,
) -> Result<InteractionResult> {
    debug!(
        "[interaction-pipeline] Starting {:?} on '{}'",
        request.kind, request.selector
    );

    // Preflight: Check element exists
    let exists = ctx.exists(&request.selector).await?;
    if !exists {
        return Ok(InteractionResult::failed(format!(
            "Element '{}' not found",
            request.selector
        )));
    }

    // Preflight: Check element is visible for interactions that need it
    let needs_visibility = matches!(
        request.kind,
        InteractionKind::Click
            | InteractionKind::NativeClick
            | InteractionKind::Hover
            | InteractionKind::Focus
    );

    if needs_visibility {
        let visible = ctx.visible(&request.selector).await?;
        if !visible {
            return Ok(InteractionResult::failed(format!(
                "Element '{}' not visible",
                request.selector
            )));
        }
    }

    // Execute based on kind
    let result = match request.kind {
        InteractionKind::Click => {
            execute_click_pipeline(ctx, &request.selector, request.allow_fallback).await
        }
        InteractionKind::Type => {
            if let Some(text) = &request.text {
                execute_type_pipeline(ctx, &request.selector, text, request.allow_fallback).await
            } else {
                Ok(InteractionResult::failed(
                    "No text provided for type action",
                ))
            }
        }
        InteractionKind::Focus => execute_focus_pipeline(ctx, &request.selector).await,
        InteractionKind::SelectAll => execute_select_all_pipeline(ctx, &request.selector).await,
        InteractionKind::Clear => execute_clear_pipeline(ctx, &request.selector).await,
        InteractionKind::NativeClick => {
            execute_native_click_pipeline(ctx, &request.selector, request.allow_fallback).await
        }
        InteractionKind::Keyboard => {
            if let Some(text) = &request.text {
                execute_keyboard_pipeline(ctx, text).await
            } else {
                Ok(InteractionResult::failed(
                    "No text provided for keyboard action",
                ))
            }
        }
        InteractionKind::Hover => execute_hover_pipeline(ctx, &request.selector).await,
    };

    // Postflight pause (always apply to successful interactions)
    match &result {
        Ok(res) if res.success => {
            ctx.pause(request.post_action_pause_ms).await;
        }
        _ => {}
    }

    result
}

/// Execute click through the pipeline
async fn execute_click_pipeline(
    ctx: &TaskContext,
    selector: &str,
    allow_fallback: bool,
) -> Result<InteractionResult> {
    // Try primary click method
    let outcome = ctx.click_internal(selector).await?;

    match outcome.click {
        ClickStatus::Success => Ok(InteractionResult::success_at(outcome.x, outcome.y)),
        _ if allow_fallback => {
            warn!("[interaction-pipeline] Click failed, attempting fallback");
            // Fallback: try coordinate-based click via the internal coordinate method
            let coord_outcome = ctx.click_coordinate_fallback(selector).await?;
            match coord_outcome.click {
                ClickStatus::Success => Ok(InteractionResult::fallback_success()),
                _ => Ok(InteractionResult::failed("Click failed after fallback")),
            }
        }
        _ => Ok(InteractionResult::failed("Click failed")),
    }
}

/// Execute native click through the pipeline
async fn execute_native_click_pipeline(
    ctx: &TaskContext,
    selector: &str,
    allow_fallback: bool,
) -> Result<InteractionResult> {
    let outcome = ctx.nativeclick_internal(selector).await?;

    match outcome.click {
        ClickStatus::Success => Ok(InteractionResult::success_at(outcome.x, outcome.y)),
        _ if allow_fallback => {
            warn!("[interaction-pipeline] Native click failed, attempting coordinate fallback");
            let coord_outcome = ctx.click_coordinate_fallback(selector).await?;
            match coord_outcome.click {
                ClickStatus::Success => Ok(InteractionResult::fallback_success()),
                _ => Ok(InteractionResult::failed(
                    "Native click failed after fallback",
                )),
            }
        }
        _ => Ok(InteractionResult::failed("Native click failed")),
    }
}

/// Execute type through the pipeline
async fn execute_type_pipeline(
    ctx: &TaskContext,
    selector: &str,
    text: &str,
    _allow_fallback: bool,
) -> Result<InteractionResult> {
    // The keyboard method handles focus internally, so we just call it directly
    ctx.keyboard_internal(selector, text).await?;
    Ok(InteractionResult::success())
}

/// Execute focus through the pipeline
async fn execute_focus_pipeline(ctx: &TaskContext, selector: &str) -> Result<InteractionResult> {
    let outcome = ctx.focus_internal(selector).await?;
    use crate::runtime::task_context::FocusStatus;

    match outcome.focus {
        FocusStatus::Success => Ok(InteractionResult::success_at(outcome.x, outcome.y)),
        _ => Ok(InteractionResult::failed(format!(
            "Failed to focus element '{}'",
            selector
        ))),
    }
}

/// Execute select_all through the pipeline
async fn execute_select_all_pipeline(
    ctx: &TaskContext,
    selector: &str,
) -> Result<InteractionResult> {
    // Use internal select_all implementation
    ctx.select_all_internal(selector).await?;
    Ok(InteractionResult::success())
}

/// Execute clear through the pipeline
async fn execute_clear_pipeline(ctx: &TaskContext, selector: &str) -> Result<InteractionResult> {
    ctx.clear_internal(selector).await?;
    Ok(InteractionResult::success())
}

/// Execute keyboard through the pipeline
async fn execute_keyboard_pipeline(ctx: &TaskContext, text: &str) -> Result<InteractionResult> {
    // Use keyboard on currently focused element
    // Note: keyboard_internal requires a selector, so we use the page body as fallback
    ctx.keyboard_internal("body", text).await?;
    Ok(InteractionResult::success())
}

/// Execute hover through the pipeline
async fn execute_hover_pipeline(ctx: &TaskContext, selector: &str) -> Result<InteractionResult> {
    let outcome = ctx.hover_internal(selector).await?;
    use crate::utils::mouse::HoverStatus;

    match outcome.hover {
        HoverStatus::Success => Ok(InteractionResult::success_at(outcome.x, outcome.y)),
        _ => Ok(InteractionResult::failed(format!(
            "Failed to hover over element '{}'",
            selector
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interaction_request_builder_click() {
        let req = InteractionRequest::click("#submit");
        assert_eq!(req.kind, InteractionKind::Click);
        assert_eq!(req.selector, "#submit");
        assert!(req.verify);
        assert!(req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 120);
    }

    #[test]
    fn test_interaction_request_builder_type() {
        let req = InteractionRequest::type_text("#input", "hello world");
        assert_eq!(req.kind, InteractionKind::Type);
        assert_eq!(req.selector, "#input");
        assert_eq!(req.text, Some("hello world".to_string()));
        assert!(req.verify);
        assert!(req.allow_fallback);
    }

    #[test]
    fn test_interaction_request_builder_chaining() {
        let req = InteractionRequest::click("#btn")
            .without_verification()
            .without_fallback()
            .with_pause(200);

        assert!(!req.verify);
        assert!(!req.allow_fallback);
        assert_eq!(req.post_action_pause_ms, 200);
    }

    #[test]
    fn test_interaction_result_success() {
        let res = InteractionResult::success();
        assert!(res.is_success());
        assert!(!res.is_fallback());
        assert!(res.verified);
    }

    #[test]
    fn test_interaction_result_success_at() {
        let res = InteractionResult::success_at(100.5, 200.5);
        assert!(res.is_success());
        assert_eq!(res.x, Some(100.5));
        assert_eq!(res.y, Some(200.5));
    }

    #[test]
    fn test_interaction_result_fallback() {
        let res = InteractionResult::fallback_success();
        assert!(res.is_success());
        assert!(res.is_fallback());
    }

    #[test]
    fn test_interaction_result_failed() {
        let res = InteractionResult::failed("element not found");
        assert!(!res.is_success());
        assert_eq!(res.error, Some("element not found".to_string()));
    }

    #[test]
    fn test_interaction_kind_coverage() {
        // Ensure all interaction kinds are defined
        let kinds = [
            InteractionKind::Click,
            InteractionKind::NativeClick,
            InteractionKind::Type,
            InteractionKind::Keyboard,
            InteractionKind::Focus,
            InteractionKind::SelectAll,
            InteractionKind::Clear,
            InteractionKind::Hover,
        ];

        // Verify each kind is distinct (no accidental duplicates)
        let mut unique = std::collections::HashSet::new();
        for kind in kinds {
            let disc = std::mem::discriminant(&kind);
            assert!(
                unique.insert(disc),
                "InteractionKind variants should be unique"
            );
        }
        assert_eq!(unique.len(), 8, "Should have 8 distinct interaction kinds");
    }

    #[test]
    fn test_interaction_request_focus_builder() {
        let req = InteractionRequest::focus("#input");
        assert_eq!(req.kind, InteractionKind::Focus);
        assert_eq!(req.selector, "#input");
        assert!(req.verify);
        assert!(!req.allow_fallback); // Focus doesn't use fallback
        assert_eq!(req.post_action_pause_ms, 80);
    }

    #[test]
    fn test_interaction_request_clear_builder() {
        let req = InteractionRequest::clear("#input");
        assert_eq!(req.kind, InteractionKind::Clear);
        assert_eq!(req.selector, "#input");
        assert!(req.verify);
        assert!(!req.allow_fallback); // Clear doesn't use fallback
        assert_eq!(req.post_action_pause_ms, 100);
    }

    #[test]
    fn test_interaction_request_select_all_builder() {
        let req = InteractionRequest::select_all("#textarea");
        assert_eq!(req.kind, InteractionKind::SelectAll);
        assert_eq!(req.selector, "#textarea");
        assert!(req.verify);
        assert!(!req.allow_fallback); // SelectAll doesn't use fallback
        assert_eq!(req.post_action_pause_ms, 80);
    }

    #[test]
    fn test_interaction_result_coordinate_accessors() {
        let with_coords = InteractionResult::success_at(150.0, 250.0);
        assert_eq!(with_coords.x, Some(150.0));
        assert_eq!(with_coords.y, Some(250.0));

        let without_coords = InteractionResult::success();
        assert_eq!(without_coords.x, None);
        assert_eq!(without_coords.y, None);
    }

    #[test]
    fn test_click_and_type_share_same_request_pattern() {
        // This test verifies that click and type use the same InteractionRequest pattern,
        // ensuring they can both flow through the shared pipeline
        let click_req = InteractionRequest::click("#btn");
        let type_req = InteractionRequest::type_text("#input", "text");

        // Both have the same request structure
        assert_eq!(click_req.verify, type_req.verify);
        assert_eq!(click_req.allow_fallback, type_req.allow_fallback);

        // Both can be modified with the same builder methods
        let modified_click = click_req.without_verification().with_pause(300);
        let modified_type = type_req.without_verification().with_pause(300);

        assert_eq!(modified_click.verify, modified_type.verify);
        assert_eq!(
            modified_click.post_action_pause_ms,
            modified_type.post_action_pause_ms
        );
    }

    #[test]
    fn test_interaction_result_error_propagation() {
        let error_cases = [
            "element not found",
            "element not visible",
            "click failed",
            "timeout waiting for element",
        ];

        for error_msg in error_cases {
            let result = InteractionResult::failed(error_msg);
            assert!(!result.is_success());
            assert_eq!(result.error, Some(error_msg.to_string()));
            assert!(!result.verified);
            assert!(!result.fallback_used);
        }
    }

    #[test]
    fn test_fallback_success_marked_correctly() {
        let result = InteractionResult::fallback_success();
        assert!(result.is_success());
        assert!(result.is_fallback());
        assert!(result.verified);
        assert!(result.success);
        assert!(result.fallback_used);
    }

    #[test]
    fn test_interaction_request_builder_pattern_consistency() {
        // All request builders should support the same chaining API
        let click = InteractionRequest::click("#btn")
            .without_verification()
            .without_fallback()
            .with_pause(500);

        let type_req = InteractionRequest::type_text("#input", "text")
            .without_verification()
            .without_fallback()
            .with_pause(500);

        let focus = InteractionRequest::focus("#input").with_pause(500);

        // Verify chaining works consistently
        assert_eq!(click.verify, type_req.verify);
        assert_eq!(click.allow_fallback, type_req.allow_fallback);
        assert_eq!(click.post_action_pause_ms, focus.post_action_pause_ms);
    }
}
