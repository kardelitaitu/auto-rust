(function() {
    var scope =
        document.querySelector('main header') ||
        document.querySelector('main [data-testid="UserProfileHeader_Items"]') ||
        document.querySelector('main') ||
        document.body;

    var buttons = scope.querySelectorAll('button, [role="button"]');
    for (var i = 0; i < buttons.length; i++) {
        var btn = buttons[i];
        var text = (btn.textContent || btn.innerText || '').trim();
        var label = (btn.getAttribute('aria-label') || '').trim();
        var dataTestId = btn.getAttribute('data-testid') || '';
        if (label.toLowerCase().includes('follow @') ||
            label.toLowerCase() === 'follow' ||
            text.toLowerCase() === 'follow' ||
            dataTestId.toLowerCase().includes('follow')) {
            var rect = btn.getBoundingClientRect();
            if (rect.width > 0 && rect.height > 0) {
                return {
                    x: rect.x + rect.width / 2,
                    y: rect.y + rect.height / 2,
                    text: text,
                    label: label
                };
            }
        }
    }
    return null;
})()