# Tutorial: Building Your First Task

A comprehensive video tutorial script for creating custom automation tasks.

## Video Metadata
- **Duration**: 20-25 minutes
- **Target Audience**: Developers ready to extend Auto-Rust
- **Prerequisites**: Completed "Getting Started" tutorial, basic Rust async knowledge
- **Outcome**: User creates and runs a custom data extraction task

---

## Section 1: Introduction (2 minutes)

### Opening Hook
"Now that you can run existing tasks, let's build something completely custom. In this tutorial, we'll create a product price tracker that navigates to an e-commerce site, extracts product information, and saves it to a file - all in about 50 lines of Rust code."

### What We'll Build
- **Task Name**: `price_tracker`
- **Functionality**:
  - Navigate to product listing page
  - Extract product names and prices
  - Handle pagination
  - Save results to JSON file
  - Track changes across runs

### Prerequisites
- Auto-Rust repository cloned and built
- Basic understanding of CSS selectors
- Familiarity with Rust async/await

---

## Section 2: Task Architecture (3 minutes)

### Task Structure Overview
"Every task in Auto-Rust follows a consistent pattern. Understanding this pattern helps you write maintainable, testable automation code."

### The Task Function Signature
```rust
use crate::runtime::task_context::TaskContext;
use anyhow::Result;
use serde_json::Value;

pub async fn my_task(
    ctx: &TaskContext,
    payload: Value
) -> Result<()> {
    // Task implementation
    Ok(())
}
```

### Key Components
1. **TaskContext**: Provides browser automation APIs
2. **Payload**: JSON configuration from CLI
3. **Result**: Success or typed error for failure

### Task File Location
```
src/task/
├── mod.rs           # Task registration
├── cookiebot.rs     # Example task
└── price_tracker.rs # Our new task ← here
```

### Task Registration
```rust
// In src/task/mod.rs
mod price_tracker;

pub async fn run_task(
    task_name: &str,
    ctx: &TaskContext,
    payload: Value
) -> Result<TaskResult> {
    match task_name {
        "price_tracker" => price_tracker::run(ctx, payload).await,
        // ... other tasks
    }
}
```

---

## Section 3: Creating the Task File (4 minutes)

### Step 1: Create File
```bash
# Create the new task file
touch src/task/price_tracker.rs
```

### Step 2: Add Module Declaration
```rust
//! Product price tracking task.
//!
//! Extracts product information from e-commerce sites and tracks
//! price changes across multiple runs.

use crate::runtime::task_context::TaskContext;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Product data structure
#[derive(Debug, Serialize, Deserialize)]
struct Product {
    name: String,
    price: String,
    url: String,
}

/// Task result with extraction metadata
#[derive(Debug, Serialize)]
struct ExtractionResult {
    timestamp: String,
    source_url: String,
    products: Vec<Product>,
    total_count: usize,
}
```

### Step 3: Main Task Function
```rust
/// Run the price tracking task.
///
/// # Arguments
/// * `ctx` - Task context for browser automation
/// * `payload` - Configuration with target URL and selectors
///
/// # Returns
/// Ok(()) on successful extraction
pub async fn run(ctx: &TaskContext, payload: Value) -> Result<()> {
    // Extract configuration from payload
    let url = payload
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://example-shop.com/products");

    println!("Starting price tracker for: {}", url);

    // Navigate to target page
    ctx.navigate(url).await?;
    ctx.wait_for_visible(".product-list").await?;

    // Extract product data
    let products = extract_products(ctx).await?;

    // Save results
    save_results(ctx, &products, url).await?;

    println!("Extracted {} products", products.len());
    Ok(())
}
```

---

## Section 4: Implementing Product Extraction (5 minutes)

### Understanding the Page Structure
"Before writing extraction code, we need to understand the page structure. Let's inspect the elements we want to capture."

### CSS Selector Strategy
```html
<!-- Example product card HTML -->
<div class="product-card" data-id="123">
  <h3 class="product-name">Wireless Headphones</h3>
  <span class="product-price">$49.99</span>
  <a class="product-link" href="/products/123">View</a>
</div>
```

### Extraction Implementation
```rust
/// Extract all products from the current page.
async fn extract_products(ctx: &TaskContext) -> Result<Vec<Product>> {
    let mut products = Vec::new();

    // Count total products
    let count = ctx.count_elements(".product-card").await?;
    println!("Found {} products on page", count);

    // Extract each product
    for i in 0..count {
        let selector = format!(".product-card:nth-child({})", i + 1);

        // Ensure element is visible
        if !ctx.is_in_viewport(&selector).await? {
            ctx.scroll_to(&selector).await?;
            ctx.pause(300).await;
        }

        // Extract product data
        let name = ctx
            .text(&format!("{} .product-name", selector))
            .await?;

        let price = ctx
            .text(&format!("{} .product-price", selector))
            .await?;

        let url = ctx
            .attr(&format!("{} .product-link", selector), "href")
            .await?;

        products.push(Product {
            name: name.trim().to_string(),
            price: price.trim().to_string(),
            url,
        });
    }

    Ok(products)
}
```

### Error Handling Pattern
```rust
// Handle missing elements gracefully
let price = match ctx.text(&format!("{} .product-price", selector)).await {
    Ok(p) => p,
    Err(_) => {
        println!("Warning: Could not extract price for item {}", i);
        "N/A".to_string()
    }
};
```

---

## Section 5: Saving and Comparing Results (4 minutes)

### Data Persistence Strategy
"We'll save results to a JSON file and compare with previous runs to detect price changes."

### Saving Results
```rust
/// Save extraction results to data file.
async fn save_results(
    ctx: &TaskContext,
    products: &[Product],
    source_url: &str,
) -> Result<()> {
    let result = ExtractionResult {
        timestamp: chrono::Utc::now().to_rfc3339(),
        source_url: source_url.to_string(),
        total_count: products.len(),
        products: products.to_vec(),
    };

    // Generate filename with timestamp
    let filename = format!(
        "results/price_tracker_{}.json",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    // Write to data directory
    ctx.write_json_data(&filename, &result).await?;
    println!("Results saved to: {}", filename);

    Ok(())
}
```

### Loading Previous Results
```rust
/// Load previous extraction results if available.
async fn load_previous_results(ctx: &TaskContext) -> Result<Option<ExtractionResult>> {
    // List all result files
    let files = ctx.list_data_files(Some("results")).await?;

    // Find most recent price tracker file
    let latest = files
        .iter()
        .filter(|f| f.starts_with("price_tracker_"))
        .max();

    if let Some(filename) = latest {
        let path = format!("results/{}", filename);
        let result: ExtractionResult = ctx.read_json_data(&path).await?;
        return Ok(Some(result));
    }

    Ok(None)
}
```

### Detecting Price Changes
```rust
/// Compare current products with previous run.
fn detect_changes(
    current: &[Product],
    previous: &[Product],
) -> Vec<PriceChange> {
    let mut changes = Vec::new();

    // Create lookup map from previous run
    let prev_map: HashMap<String, String> = previous
        .iter()
        .map(|p| (p.name.clone(), p.price.clone()))
        .collect();

    for product in current {
        if let Some(old_price) = prev_map.get(&product.name) {
            if old_price != &product.price {
                changes.push(PriceChange {
                    product: product.name.clone(),
                    old_price: old_price.clone(),
                    new_price: product.price.clone(),
                });
            }
        }
    }

    changes
}
```

---

## Section 6: Adding Pagination Support (3 minutes)

### Handling Multiple Pages
"Most e-commerce sites paginate their product listings. Let's handle that gracefully."

### Pagination Detection
```rust
/// Check if there's a next page and navigate to it.
async fn has_next_page(ctx: &TaskContext) -> Result<bool> {
    // Check for next page button/link
    let has_next = ctx.exists(".pagination .next-page").await?;

    if has_next {
        // Check if it's disabled (last page)
        let disabled = ctx
            .attr(".pagination .next-page", "disabled")
            .await
            .is_ok();

        return Ok(!disabled);
    }

    Ok(false)
}
```

### Pagination Loop
```rust
/// Extract products from all pages.
async fn extract_all_pages(ctx: &TaskContext) -> Result<Vec<Product>> {
    let mut all_products = Vec::new();
    let mut page_count = 0;

    loop {
        println!("Processing page {}", page_count + 1);

        // Extract from current page
        let products = extract_products(ctx).await?;
        all_products.extend(products);

        // Check for next page
        if !has_next_page(ctx).await? {
            break;
        }

        // Navigate to next page
        ctx.click(".pagination .next-page").await?;
        ctx.wait_for_visible(".product-list").await?;
        ctx.pause(1000).await; // Let page settle

        page_count += 1;

        // Safety limit
        if page_count >= 10 {
            println!("Reached maximum page limit");
            break;
        }
    }

    println!("Extracted from {} pages", page_count + 1);
    Ok(all_products)
}
```

---

## Section 7: Testing Your Task (3 minutes)

### Unit Testing
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_serialization() {
        let product = Product {
            name: "Test Product".to_string(),
            price: "$19.99".to_string(),
            url: "/products/1".to_string(),
        };

        let json = serde_json::to_string(&product).unwrap();
        assert!(json.contains("Test Product"));
        assert!(json.contains("19.99"));
    }

    #[test]
    fn test_price_change_detection() {
        let previous = vec![
            Product {
                name: "Widget".to_string(),
                price: "$10.00".to_string(),
                url: "/1".to_string(),
            },
        ];

        let current = vec![
            Product {
                name: "Widget".to_string(),
                price: "$12.00".to_string(),
                url: "/1".to_string(),
            },
        ];

        let changes = detect_changes(&current, &previous);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].old_price, "$10.00");
        assert_eq!(changes[0].new_price, "$12.00");
    }
}
```

### Integration Testing
```bash
# Run the task with test URL
cargo run price_tracker url=https://example-shop.com/products

# Check output
ls data/results/price_tracker_*.json

# View results
cat data/results/price_tracker_20240115_103000.json
```

---

## Section 8: Advanced Features (3 minutes)

### Adding Configuration Options
```rust
/// Task configuration from payload
#[derive(Debug, Deserialize)]
struct Config {
    url: String,
    #[serde(default = "default_max_pages")]
    max_pages: usize,
    #[serde(default)]
    selectors: ProductSelectors,
}

#[derive(Debug, Deserialize, Default)]
struct ProductSelectors {
    #[serde(default = "default_product_card")]
    product_card: String,
    #[serde(default = "default_product_name")]
    product_name: String,
    #[serde(default = "default_product_price")]
    product_price: String,
}

fn default_max_pages() -> usize { 10 }
fn default_product_card() -> String { ".product-card".to_string() }
fn default_product_name() -> String { ".product-name".to_string() }
fn default_product_price() -> String { ".product-price".to_string() }
```

### Using with Custom Selectors
```bash
# Run with custom selectors
cargo run price_tracker '
{
  "url": "https://shop.example.com",
  "selectors": {
    "product_card": ".item-card",
    "product_name": "h2.title",
    "product_price": ".price-tag"
  }
}'
```

### Adding Retry Logic
```rust
/// Extract with automatic retry on failure.
async fn extract_with_retry(
    ctx: &TaskContext,
    max_attempts: u32,
) -> Result<Vec<Product>> {
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match extract_all_pages(ctx).await {
            Ok(products) => return Ok(products),
            Err(e) => {
                println!("Attempt {} failed: {}", attempt, e);
                last_error = Some(e);

                if attempt < max_attempts {
                    ctx.pause(2000 * attempt as u64).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(||
        anyhow::anyhow!("All retry attempts failed")
    ))
}
```

---

## Section 9: Conclusion (2 minutes)

### What We Built
✅ Complete price tracking task from scratch
✅ Product extraction with CSS selectors
✅ Pagination handling
✅ Data persistence with JSON
✅ Price change detection
✅ Configurable selectors
✅ Unit and integration tests

### Key Takeaways
1. **Task Pattern**: Context + Payload → Result
2. **Error Handling**: Graceful degradation over hard failures
3. **Testing**: Unit test logic, integration test browser interaction
4. **Configuration**: JSON payload for flexible task behavior

### Next Steps
- **Add Notifications**: Send email/Slack on price changes
- **Schedule Runs**: Use cron to run task periodically
- **Database Storage**: Persist to SQLite or PostgreSQL
- **Web Dashboard**: Build UI to view price history

### Resources
- **Complete Code**: `src/task/price_tracker.rs` in repository
- **API Reference**: `docs/API_REFERENCE.md`
- **Community**: GitHub Discussions for questions

---

## Production Notes for Video Creator

### Visual Aids Needed
- [ ] Split-screen: Code editor + browser showing page structure
- [ ] Browser DevTools showing element inspection
- [ ] Terminal showing task execution and JSON output
- [ ] File tree showing task file location
- [ ] Diagram: Task flow from CLI to browser to file

### Code Samples to Have Ready
- Pre-written HTML fixture for testing
- Sample product page for demonstration
- Test JSON payloads for different scenarios
- Example output JSON files

### Timing Tips
- Use fast-forward for compilation steps
- Show browser in slow-motion during interactions
- Pause on important code sections
- Use callouts for key concepts

### Common Pitfalls to Mention
- Selector specificity (avoid overly generic selectors)
- Timing issues (wait for elements before interaction)
- Rate limiting (add delays between requests)
- Dynamic content (handle JavaScript-rendered elements)

---

*End of Tutorial Script*
