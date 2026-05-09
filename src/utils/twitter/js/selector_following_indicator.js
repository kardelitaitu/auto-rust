(function() {
    var buttons = document.querySelectorAll('button');
    for (var i = 0; i < buttons.length; i++) {
        var btn = buttons[i];
        var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
        var label = (btn.getAttribute('aria-label') || '').toLowerCase();
        var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();
        if (text === 'following' ||
            label.includes('following @') ||
            dataTestId.includes('unfollow')) {
            return true;
        }
    }
    return false;
})()