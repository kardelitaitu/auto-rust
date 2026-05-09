(function() {
    function visible(el) {
        if (!el) return false;
        var rect = el.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
    }
    function center(el) {
        var rect = el.getBoundingClientRect();
        return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }

    var modal = document.querySelector('div[role="dialog"]');
    var articles = [];
    if (modal && visible(modal)) {
        articles = Array.prototype.slice.call(
            modal.querySelectorAll('article[data-testid="tweet"]')
        ).filter(visible);
    }

    if (articles.length === 0) {
        articles = Array.prototype.slice.call(
            document.querySelectorAll('article[data-testid="tweet"]')
        ).filter(visible);
    }

    var statusMatch = window.location.pathname.match(/\/status\/(\d+)/);
    var targetStatusId = statusMatch ? statusMatch[1] : null;
    var targetArticle = null;
    if (targetStatusId) {
        for (var i = 0; i < articles.length; i++) {
            if (articles[i].querySelector('a[href*="/status/' + targetStatusId + '"]')) {
                targetArticle = articles[i];
                break;
            }
        }
    }
    var scopes = articles.length > 0
        ? [targetArticle || articles[0]]
        : [document.querySelector('main'), document.body].filter(Boolean);

    for (var i = 0; i < scopes.length; i++) {
        var button = scopes[i].querySelector("{SELECTOR}");
        if (visible(button)) return center(button);
    }
    return null;
})()