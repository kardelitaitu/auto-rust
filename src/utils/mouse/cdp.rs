//! CDP mouse event dispatching utilities.
//!
//! Low-level functions for mapping mouse buttons/events to Chrome DevTools Protocol
//! equivalents, dispatching mouse events via CDP or JS fallback, and pointer event injection.

use super::types::MouseButton;
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton as CdpMouseButton,
};
use chromiumoxide::Page;
use log::debug;
use tokio::time::{timeout, Duration};

fn map_cdp_button(button_idx: u16) -> Option<CdpMouseButton> {
    match button_idx {
        0 => Some(CdpMouseButton::Left),
        1 => Some(CdpMouseButton::Middle),
        2 => Some(CdpMouseButton::Right),
        _ => None,
    }
}

pub fn mouse_button_mask(button_idx: u16) -> i64 {
    match button_idx {
        0 => 1, // Left
        1 => 4, // Middle
        2 => 2, // Right
        _ => 0,
    }
}

/// Dispatch a pointer event for enhanced browser event coverage.
/// Pointer events (pointerover, pointerenter, pointermove, etc.) are fired
/// by real browsers but often skipped by automation tools.
pub async fn dispatch_pointer_event(
    page: &Page,
    event_type: &str,
    x: f64,
    y: f64,
    button: MouseButton,
) -> Result<()> {
    let pointer_id = 1; // Standard mouse pointer ID
    let button_idx = button.as_button_index();
    let buttons = mouse_button_mask(button_idx);

    let js = format!(
        r"(function() {{
            const el = document.elementFromPoint({x}, {y});
            if (!el) return false;
            
            const evt = new PointerEvent('{event_type}', {{
                bubbles: true,
                cancelable: true,
                clientX: {x},
                clientY: {y},
                pointerId: {pointer_id},
                width: 1,
                height: 1,
                pressure: 0.5,
                tiltX: 0,
                tiltY: 0,
                pointerType: 'mouse',
                isPrimary: true,
                button: {button_idx},
                buttons: {buttons}
            }});
            
            el.dispatchEvent(evt);
            return true;
        }})();"
    );

    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_pointer_event timed out"))??;

    let did_dispatch = result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !did_dispatch {
        // Non-fatal - some elements don't support pointer events
        debug!("dispatch_pointer_event: no element at ({x}, {y})");
    }

    Ok(())
}

pub(crate) fn map_cdp_event_type(event_type: &str) -> Option<DispatchMouseEventType> {
    match event_type {
        "mousedown" => Some(DispatchMouseEventType::MousePressed),
        "mouseup" => Some(DispatchMouseEventType::MouseReleased),
        "mousemove" => Some(DispatchMouseEventType::MouseMoved),
        _ => None,
    }
}

pub async fn dispatch_mouse_event_cdp(
    page: &Page,
    event_type: DispatchMouseEventType,
    x: f64,
    y: f64,
    button: Option<CdpMouseButton>,
    buttons: Option<i64>,
    click_count: Option<i64>,
) -> Result<()> {
    let params = DispatchMouseEventParams {
        r#type: event_type,
        x,
        y,
        modifiers: None,
        timestamp: None,
        button,
        buttons,
        click_count,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
        delta_x: None,
        delta_y: None,
        pointer_type: None,
    };
    timeout(Duration::from_secs(2), page.execute(params))
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_mouse_event_cdp timed out"))??;
    Ok(())
}

/// Dispatch a single mouse event using CDP, with JS fallback.
pub async fn dispatch_single_mouse_event(
    page: &Page,
    x: f64,
    y: f64,
    button_idx: u16,
    event_type: &str,
) -> Result<()> {
    // Try CDP dispatch first
    if let Some(cdp_type) = map_cdp_event_type(event_type) {
        let button = map_cdp_button(button_idx);
        let buttons = match cdp_type {
            DispatchMouseEventType::MousePressed => Some(mouse_button_mask(button_idx)),
            DispatchMouseEventType::MouseReleased => Some(0),
            DispatchMouseEventType::MouseMoved | DispatchMouseEventType::MouseWheel => None,
        };
        let click_count = if matches!(
            cdp_type,
            DispatchMouseEventType::MousePressed | DispatchMouseEventType::MouseReleased
        ) {
            Some(1)
        } else {
            None
        };

        if dispatch_mouse_event_cdp(page, cdp_type, x, y, button, buttons, click_count)
            .await
            .is_ok()
        {
            return Ok(());
        }
    } else if event_type == "click" {
        // Native click is produced by mousePressed + mouseReleased.
        return Ok(());
    }

    // Fallback to JS dispatch
    let eval = page.evaluate(format!(
        "(function() {{
            const el = document.elementFromPoint({x}, {y});
            if (!el) return false;

            const evt = new MouseEvent('{event_type}', {{
                bubbles: true,
                cancelable: true,
                clientX: {x},
                clientY: {y},
                button: {button_idx}
            }});
            el.dispatchEvent(evt);
            return true;
        }})();"
    ));

    let result = timeout(Duration::from_secs(2), eval)
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_single_mouse_event timed out"))??;

    let did_dispatch = result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !did_dispatch {
        anyhow::bail!("dispatch_single_mouse_event found no element at ({x:.1},{y:.1})");
    }

    Ok(())
}

pub async fn dispatch_mouse_action(
    page: &Page,
    x: f64,
    y: f64,
    button_idx: u16,
    event_type: &str,
) -> Result<()> {
    dispatch_single_mouse_event(page, x, y, button_idx, event_type).await
}

#[cfg(test)]
mod tests {
    use super::{map_cdp_button, map_cdp_event_type, mouse_button_mask};
    use chromiumoxide::cdp::browser_protocol::input::{
        DispatchMouseEventType, MouseButton as CdpMouseButton,
    };

    #[test]
    fn map_cdp_button_all_valid_indices() {
        assert_eq!(map_cdp_button(0), Some(CdpMouseButton::Left));
        assert_eq!(map_cdp_button(1), Some(CdpMouseButton::Middle));
        assert_eq!(map_cdp_button(2), Some(CdpMouseButton::Right));
    }

    #[test]
    fn map_cdp_button_unknown_index_is_none() {
        assert_eq!(map_cdp_button(3), None);
        assert_eq!(map_cdp_button(255), None);
    }

    #[test]
    fn mouse_button_mask_standard_masks() {
        assert_eq!(mouse_button_mask(0), 1); // Left
        assert_eq!(mouse_button_mask(1), 4); // Middle
        assert_eq!(mouse_button_mask(2), 2); // Right
    }

    #[test]
    fn mouse_button_mask_unknown_is_zero() {
        assert_eq!(mouse_button_mask(7), 0);
    }

    #[test]
    fn map_cdp_event_type_known_events() {
        assert_eq!(
            map_cdp_event_type("mousedown"),
            Some(DispatchMouseEventType::MousePressed)
        );
        assert_eq!(
            map_cdp_event_type("mouseup"),
            Some(DispatchMouseEventType::MouseReleased)
        );
        assert_eq!(
            map_cdp_event_type("mousemove"),
            Some(DispatchMouseEventType::MouseMoved)
        );
    }

    #[test]
    fn map_cdp_event_type_unknown_events() {
        assert_eq!(map_cdp_event_type("mouseenter"), None);
        assert_eq!(map_cdp_event_type("click"), None);
        assert_eq!(map_cdp_event_type(""), None);
    }
}
