last audited 16-06-26 by opencode

this document is the text human-readable list of ./src/runtime/task_context/

format:

**function_name(args)** -- short description
```rust
api.function_name(real_args).await?;
```

## Task API Verbs

**navigate(url: &str, timeout_ms: u64)** -- Navigate to a URL
```rust
api.navigate("https://example.com", 30_000).await?;
```

**iframe(selector: &str, timeout_ms: u64)** -- Enter an iframe by navigating the tab to its src URL
```rust
let miniapp_url = api.iframe("iframe", 30_000).await?;
```

**iframe_click(iframe_selector: &str, element_selector: &str, timeout_ms: u64)** -- Click an element inside an iframe in place (no navigation, no new tab). iframe_selector accepts CSS or XPath (starts with /)
```rust
let outcome = api.iframe_click("iframe", "#btn-go", 30_000).await?;
let outcome = api.iframe_click("/html/body/div[10]/div/div[2]/div/div/iframe", "#btn-go", 30_000).await?;
```

**check_page_connected()** -- Check if page is connected
```rust
api.check_page_connected().await?;
```

**screenshot()** -- Capture screenshot (quality default)
```rust
let data = api.screenshot().await?;
```

**screenshot_with_quality(quality: u8)** -- Capture screenshot with quality (0-100)
```rust
let data = api.screenshot_with_quality(85).await?;
```

**export_cookies(url: &str)** -- Export all cookies for URL
```rust
let cookies = api.export_cookies("https://example.com").await?;
```

**export_cookies_for_domain(domain: &str)** -- Export cookies for specific domain
```rust
let cookies = api.export_cookies_for_domain("example.com").await?;
```

**export_session_cookies(url: &str)** -- Export session cookies only
```rust
let cookies = api.export_session_cookies("https://example.com").await?;
```

**has_cookie(name: &str, domain: Option<&str>)** -- Check if cookie exists
```rust
let exists = api.has_cookie("session_id", Some("example.com")).await?;
```

**import_cookies(cookies: &[serde_json::Value])** -- Import cookies into browser
```rust
api.import_cookies(&cookies).await?;
```

**http_get(url: &str)** -- Make HTTP GET request
```rust
let resp = api.http_get("https://api.example.com/data").await?;
```

**http_post_json(url: &str, body: &T)** -- Make HTTP POST request with JSON body
```rust
let resp = api.http_post_json("https://api.example.com", &payload).await?;
```

**download_file(url: &str, relative_path: &str)** -- Download file to relative path
```rust
api.download_file("https://example.com/file.zip", "downloads/file.zip").await?;
```

**export_session(url: &str)** -- Export session data (cookies + localStorage)
```rust
let session = api.export_session("https://example.com").await?;
```

**get_computed_style(selector: &str, property: &str)** -- Get computed CSS style of element
```rust
let color = api.get_computed_style("#button", "color").await?;
```

**get_element_rect(selector: &str)** -- Get element bounding rect
```rust
let rect = api.get_element_rect("#submit").await?;
```

**get_scroll_position()** -- Get current page scroll position
```rust
let (x, y) = api.get_scroll_position().await?;
```

**count_elements(selector: &str)** -- Count elements matching selector
```rust
let count = api.count_elements(".item").await?;
```

**is_in_viewport(selector: &str)** -- Check if element is in visible viewport
```rust
let visible = api.is_in_viewport("#hero").await?;
```

**export_browser(url: &str)** -- Export complete browser data (cookies + storage)
```rust
let data = api.export_browser("https://example.com").await?;
```

**import_browser(url: &str, data: &BrowserData)** -- Import complete browser data
```rust
api.import_browser("https://example.com", &data).await?;
```

**wait_for_load(timeout_ms: u64)** -- Wait for page load complete
```rust
api.wait_for_load(30_000).await?;
```

**focus(selector: &str)** -- Focus an element
```rust
api.focus("#username").await?;
```

**hover(selector: &str)** -- Hover over an element
```rust
api.hover("#menu-item").await?;
```

**move_mouse_to(x: f64, y: f64)** -- Move mouse to absolute coordinates
```rust
api.move_mouse_to(100.0, 200.0).await?;
```

**move_mouse_fast(x: f64, y: f64)** -- Move mouse quickly to coordinates
```rust
api.move_mouse_fast(100.0, 200.0).await?;
```

**randomcursor()** -- Move cursor with random human-like path
```rust
api.randomcursor().await?;
```

**sync_cursor_overlay()** -- Sync cursor overlay position
```rust
api.sync_cursor_overlay().await?;
```

**click_at(x: f64, y: f64)** -- Click at absolute coordinates
```rust
api.click_at(100.0, 200.0).await?;
```

**click(selector: &str)** -- Click element with human-like cursor movement
```rust
api.click("#submit-button").await?;
```

**double_click(selector: &str)** -- Double-click an element
```rust
api.double_click("#item").await?;
```

**middle_click(selector: &str)** -- Middle-click an element
```rust
api.middle_click("#link").await?;
```

**right_click(selector: &str)** -- Right-click an element
```rust
api.right_click("#context-menu").await?;
```

**drag(from_selector: &str, to_selector: &str)** -- Drag from one element to another
```rust
api.drag("#handle", "#drop-zone").await?;
```

**press(key: &str)** -- Press a keyboard key
```rust
api.press("Enter").await?;
```

**type(selector: &str, text: &str)** -- Type text into element with human-like timing
```rust
api.type("#input", "Hello World").await?;
```

**wait_for(selector: &str, timeout_ms: u64)** -- Wait for element to appear
```rust
api.wait_for("#content", 5_000).await?;
```

**exists(selector: &str)** -- Check if element exists in DOM
```rust
let exists = api.exists("#modal").await?;
```

**visible(selector: &str)** -- Check if element is visible
```rust
let visible = api.visible("#button").await?;
```

**text(selector: &str)** -- Get element text content
```rust
let text = api.text("#title").await?;
```

**html(selector: &str)** -- Get element inner HTML
```rust
let html = api.html("#content").await?;
```

**attr(selector: &str, attribute: &str)** -- Get element attribute value
```rust
let href = api.attr("#link", "href").await?;
```

**pause(duration_ms: u64)** -- Uniform-random pause with ~20% spread
```rust
api.pause(1_000).await?;
```

**pause_with_variance(duration_ms: u64, variance_pct: u8)** -- Pause with custom variance
```rust
api.pause_with_variance(1_000, 30).await?;
```

**pause_human(duration_ms: u64)** -- Gaussian pause for human-like timing
```rust
api.pause_human(2_000).await?;
```