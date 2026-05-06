# Quality Rules

- The JS injected into the page must be robust against missing elements (use optional chaining or explicit checks).
- Performance: The visibility and presence checks in the DOM must be efficient to avoid lagging the browser.
- No regression: Ensure the home feed navigation still works correctly and doesn't accidentally think it's on a tweet page when just a small unrelated popup appears.
