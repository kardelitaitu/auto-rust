(function() {
    var x = {X};
    var y = {Y};
    var tweetArticle = document.querySelector('article[data-testid="tweet"]');
    var root = tweetArticle || document;
    var controls = root.querySelectorAll('button[data-testid], a[data-testid]');
    var nearest = null;
    var best = Number.POSITIVE_INFINITY;
    for (var i = 0; i < controls.length; i++) {
        var el = controls[i];
        var testId = (el.getAttribute('data-testid') || '').toLowerCase();
        if (!(testId.includes('like') || testId.includes('unlike'))) continue;
        var rect = el.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) continue;
        var cx = rect.x + rect.width / 2;
        var cy = rect.y + rect.height / 2;
        var dist = Math.hypot(cx - x, cy - y);
        if (dist < best) {
            best = dist;
            nearest = el;
        }
    }

    if (!nearest || best > 120) return false;
    var nearestId = (nearest.getAttribute('data-testid') || '').toLowerCase();
    if (nearestId.includes('unlike')) return true;

    var svg = nearest.querySelector('svg');
    if (!svg) return false;
    var color = (svg.getAttribute('color') || svg.getAttribute('fill') || '').toLowerCase();
    return color.includes('rgb') || color.includes('#');
})()
