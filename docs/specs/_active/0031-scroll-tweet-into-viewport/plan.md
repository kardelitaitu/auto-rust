last audited 26-06-26 by antigravity

# Plan: Scroll Tweet into Viewport and Add Retweet Fallback

## Baseline
- Position-based engagements from the feed (like, retweet, bookmark, follow) use coordinate positions scraped at candidate scan time.
- If a tweet is located below the active viewport, these coordinates are outside the clickable viewport area, causing clicks to fail or hit wrong elements.
- `retweet_at_position` lacks a fallback selector click. If the confirm button coordinate search fails, the entire retweet action fails immediately.

## Proposed Changes

### 1. New JS Helper: `src/utils/twitter/js/js_scroll_and_get_tweet_button.js`
Create a new JS script to find a tweet by `tweet_id`, scroll it into the center of the viewport, and return the center coordinates of the target button (e.g., `"like"`, `"retweet"`, `"bookmark"`, `"follow"`).

```javascript
(function() {
    var tweetId = "{TWEET_ID}";
    var buttonName = "{BUTTON_NAME}";
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    if (articles.length === 0) articles = document.querySelectorAll('article');
    
    for (var i = 0; i < articles.length; i++) {
        var el = articles[i];
        
        var links = el.querySelectorAll('a[href*="/status/"]');
        var statusUrl = null;
        for (var j = 0; j < links.length; j++) {
            var href = links[j].getAttribute('href');
            var parts = href.split('/').filter(function(p) { return p.length > 0; });
            if (parts.length === 3 && parts[1] === 'status' && !isNaN(parts[2])) {
                statusUrl = href;
                break;
            }
        }
        var statusId = null;
        if (statusUrl) {
            var statusParts = statusUrl.split('/').filter(function(p) { return p.length > 0; });
            statusId = statusParts[statusParts.length - 1].split(/[?#]/)[0];
        }
        var currentId = el.dataset.tweetId ||
                        el.getAttribute('data-item-id') ||
                        el.getAttribute('data-tweet-id') ||
                        statusId;
                        
        if (currentId === tweetId) {
            // Scroll the tweet into the center of the viewport
            el.scrollIntoView({ block: 'center', behavior: 'instant' });
            
            // Query target button
            var btn = el.querySelector('[data-testid="' + buttonName + '"]');
            if (btn) {
                var rect = btn.getBoundingClientRect();
                return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
            }
        }
    }
    return null;
})()
```

### 2. Update Selectors: `src/utils/twitter/twitteractivity_selectors.rs`
Expose the new script:
```rust
/// Returns JS to scroll a tweet into view and find a button's center coordinates.
#[must_use]
pub fn js_scroll_and_get_tweet_button() -> &'static str {
    include_str!("js/js_scroll_and_get_tweet_button.js")
}
```

### 3. Update Action dispatching: `src/utils/twitter/engagement/dispatch.rs`
For position-based actions (like, retweet, follow, bookmark), instead of directly using `extract_tweet_button_position` on the cached `tweet` coordinates, evaluate `js_scroll_and_get_tweet_button` on the page first.
- Re-retrieve the target button coordinate by dynamically scrolling and locating it on the page.
- If the new coordinates are successfully retrieved, use them for the action. Otherwise, fall back to the cached coordinates.

Example helper in `dispatch.rs`:
```rust
async fn scroll_and_get_button_pos(
    api: &TaskContext,
    tweet_id: &TweetId,
    button_name: &str,
) -> Option<(f64, f64)> {
    let js = twitteractivity_selectors::js_scroll_and_get_tweet_button()
        .replace("{TWEET_ID}", tweet_id.as_str())
        .replace("{BUTTON_NAME}", button_name);
    match api.page().evaluate(js).await {
        Ok(res) => res.value().and_then(|v| {
            let obj = v.as_object()?;
            let x = obj.get("x")?.as_f64()?;
            let y = obj.get("y")?.as_f64()?;
            Some((x, y))
        }),
        Err(e) => {
            log::warn!("[dispatch] Failed to resolve button position after scroll: {e}");
            None
        }
    }
}
```

### 4. Fallback in `retweet_at_position`: `src/utils/twitter/twitteractivity_actions.rs`
Implement a fallback direct selector click if coordinate confirmation fails:
```rust
    if let Some((cx, cy)) = result.value().and_then(parse_button_coords) {
        api.move_mouse_to(cx, cy).await?;
        human_pause(api, 250).await;
        api.click_at(cx, cy).await?;
        human_pause(api, 800).await;
        return Ok(EngagementOutcome::Completed);
    }

    // Fallback: direct click using selector
    warn!("[retweet] Coordinate confirm search failed, attempting fallback selector click...");
    if let Err(e) = api.scroll_into_view(twitteractivity_selectors::RETWEET_CONFIRM_SELECTOR).await {
        info!("[retweet] Failed to scroll retweet confirm button into view: {e}");
        return Ok(EngagementOutcome::Failed);
    }
    if let Err(e) = api.click(twitteractivity_selectors::RETWEET_CONFIRM_SELECTOR).await {
        info!("[retweet] Failed to click retweet confirm: {e}");
        return Ok(EngagementOutcome::Failed);
    }
    Ok(EngagementOutcome::Completed)
```

## Rationale
- Scrolling the tweet to the center ensures that coordinates are well within the clickable viewport.
- Re-querying coordinates immediately after the scroll ensures that we click the exact actual layout coordinates rather than stale ones from scan time.
- Adding a selector click fallback in `retweet_at_position` ensures that even if mouse coordinate clicks fail (due to layout shifts or browser popups), the retweet action successfully falls back and recovers.
