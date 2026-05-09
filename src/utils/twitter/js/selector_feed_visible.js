(function() {
    // Prefer data-testid attributes (most stable)
    if (document.querySelector('[data-testid="primaryColumn"]')) return true;
    if (document.querySelector('main[role="main"]')) return true;
    // Fallback to article detection
    if (document.querySelector('article[data-testid="tweet"]')) return true;
    if (document.querySelector('article')) return true;
    return false;
})()