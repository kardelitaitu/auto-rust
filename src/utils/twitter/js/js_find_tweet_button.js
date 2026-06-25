(function() {
    var buttons = document.querySelectorAll('button[data-testid="tweetButton"]');
    var fallback = null;
    for (var i = 0; i < buttons.length; i++) {
        var btn = buttons[i];
        var rect = btn.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) continue;
        if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
        
        if (!fallback) {
            fallback = { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        }
        
        var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
        if (text === 'post' || text === 'reply' || text === 'reply all' || text === 'post reply') {
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        }
    }
    return fallback;
})()
