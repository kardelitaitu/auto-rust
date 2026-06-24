(function() {
    var selectors = ['button[data-testid="tweetButtonInline"]', 'button[data-testid="tweetButton"]'];
    for (var s = 0; s < selectors.length; s++) {
        var buttons = document.querySelectorAll(selectors[s]);
        for (var i = 0; i < buttons.length; i++) {
            var btn = buttons[i];
            var rect = btn.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) continue;
            if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
            var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
            if (text !== 'reply' && text !== 'post' && text !== 'reply all' && text !== 'post reply') continue;
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        }
    }
    return null;
})()
