(function() {
    function visible(el) {
        if (!el) return false;
        var rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
    }
    
    // Look for dropdown/dialog container first
    var scopes = Array.prototype.slice.call(
        document.querySelectorAll('[role="menu"], div[role="dialog"], [data-testid="Dropdown"]')
    ).filter(visible);
    if (scopes.length === 0) scopes = [document.body];

    for (var s = 0; s < scopes.length; s++) {
        // Try exact data-testid first
        var exact = scopes[s].querySelector('[data-testid="retweetConfirm"]');
        if (exact) {
            if (!visible(exact)) {
                exact.scrollIntoView({ block: 'center', behavior: 'instant' });
            }
            if (visible(exact)) return center(exact);
        }
        
        // Fallback to text matching
        var items = scopes[s].querySelectorAll('[role="button"], [role="menuitem"], div, span');
        for (var i = 0; i < items.length; i++) {
            var el = items[i];
            var ariaLabel = el.getAttribute('aria-label') || '';
            var text = el.textContent || el.innerText || '';
            var haystack = (ariaLabel + ' ' + text).toLowerCase();
            
            // Check for retweet (but not quote or undo/unretweet)
            if (haystack.includes('retweet') && !haystack.includes('quote') && !haystack.includes('undo') && !haystack.includes('unretweet')) {
                if (!visible(el)) {
                    el.scrollIntoView({ block: 'center', behavior: 'instant' });
                }
                if (visible(el)) return center(el);
            }
        }
    }
    return null;

    function center(el) {
        var rect = el.getBoundingClientRect();
        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
    }
})()
