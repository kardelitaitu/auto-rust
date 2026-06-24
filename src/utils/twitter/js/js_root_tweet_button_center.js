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
    
    // 1. Get ALL articles in DOM (visible or not)
    var allArticles = [];
    if (modal) {
        allArticles = Array.prototype.slice.call(modal.querySelectorAll('article[data-testid="tweet"]'));
    }
    if (allArticles.length === 0) {
        allArticles = Array.prototype.slice.call(document.querySelectorAll('article[data-testid="tweet"]'));
    }

    // 2. Identify the target root tweet article
    var statusMatch = window.location.pathname.match(/\/status\/(\d+)/);
    var targetStatusId = statusMatch ? statusMatch[1] : null;
    var targetArticle = null;

    if (targetStatusId) {
        // Try to match by status ID first
        for (var i = 0; i < allArticles.length; i++) {
            if (allArticles[i].querySelector('a[href*="/status/' + targetStatusId + '"]')) {
                targetArticle = allArticles[i];
                break;
            }
        }
    }

    // Fallback: on a status page, the first article in the DOM is the root tweet
    if (!targetArticle && targetStatusId && allArticles.length > 0) {
        targetArticle = allArticles[0];
    }

    // 3. If we found the target root article, make sure it is scrolled into view so its buttons are mounted/visible
    if (targetArticle) {
        var rect = targetArticle.getBoundingClientRect();
        var isPartiallyVisible = (rect.top < window.innerHeight && rect.bottom >= 0);
        if (!isPartiallyVisible || rect.height <= 0 || rect.width <= 0) {
            targetArticle.scrollIntoView({ block: 'center', behavior: 'instant' });
        }
    }

    // 4. Now find the button within the target article (or fallback to visible articles if no targetArticle found)
    var scopes = [];
    if (targetArticle) {
        scopes.push(targetArticle);
    } else {
        var visibleArticles = allArticles.filter(visible);
        if (visibleArticles.length > 0) {
            scopes.push(visibleArticles[0]);
        } else {
            var mainEl = document.querySelector('main');
            if (mainEl) scopes.push(mainEl);
            scopes.push(document.body);
        }
    }

    for (var i = 0; i < scopes.length; i++) {
        var button = scopes[i].querySelector("{SELECTOR}");
        // If the button is not visible, try scrolling it into view
        if (button && !visible(button)) {
            button.scrollIntoView({ block: 'center', behavior: 'instant' });
        }
        if (visible(button)) return center(button);
    }
    return null;
})()