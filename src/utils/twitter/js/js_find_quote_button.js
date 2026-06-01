(function() {
    function visible(el) {
        if (!el) return false;
        var rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
    }
    var scopes = Array.prototype.slice.call(
        document.querySelectorAll('[role="menu"], div[role="dialog"], [data-testid="Dropdown"]')
    ).filter(visible);
    if (scopes.length === 0) scopes = [document.body];

    var buttons = [];
    for (var s = 0; s < scopes.length; s++) {
        var exact = scopes[s].querySelector('a[href="/compose/post"][role="menuitem"]');
        var exactText = exact ? (exact.textContent || exact.innerText || '').trim().toLowerCase() : '';
        if (visible(exact) && exactText.includes('quote')) {
            return center(exact);
        }
        buttons = buttons.concat(Array.prototype.slice.call(scopes[s].querySelectorAll('[role="button"], [role="menuitem"]')));
    }
    for (var i = 0; i < buttons.length; i++) {
        var btn = buttons[i];
        var ariaLabel = btn.getAttribute('aria-label') || '';
        var text = btn.textContent || btn.innerText || '';
        var haystack = (ariaLabel + ' ' + text).toLowerCase();
        if (haystack.includes('quote')) {
            if (visible(btn)) return center(btn);
        }
    }
    return null;

    function center(el) {
        var rect = el.getBoundingClientRect();
        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
    }
})()
