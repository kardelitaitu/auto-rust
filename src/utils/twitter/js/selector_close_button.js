(function() {
    var closeButtons = document.querySelectorAll('button[aria-label*="Close"], button[data-testid*="close"], div[role="button"][aria-label*="Close"]');
    for (var i = 0; i < closeButtons.length; i++) {
        var btn = closeButtons[i];
        var rect = btn.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
            return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
        }
    }
    return null;
})()