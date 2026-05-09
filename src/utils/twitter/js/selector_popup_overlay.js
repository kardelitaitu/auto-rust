(function() {
    var selectors = [
        'div[role="dialog"]',
        'div[aria-modal="true"]',
        'div[data-testid="sidebarColumn"]',
        'div[data-testid="app-bar-ads"]',
        'div[data-testid="placementTracking"]',
        'div[aria-label=" cookie"]',
        'div[aria-label="Privacy"]'
    ];
    for (var i = 0; i < selectors.length; i++) {
        var el = document.querySelector(selectors[i]);
        if (el) {
            var rect = el.getBoundingClientRect();
            if (rect.width > 100 && rect.height > 100) {
                return el;
            }
        }
    }
    return null;
})()