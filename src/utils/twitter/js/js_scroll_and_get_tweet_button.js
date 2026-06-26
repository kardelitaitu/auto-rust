(function() {
    var tweetId = "{TWEET_ID}";
    var buttonName = "{BUTTON_NAME}";
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    if (articles.length === 0) articles = document.querySelectorAll('article');
    
    for (var i = 0; i < articles.length; i++) {
        var el = articles[i];
        
        var links = el.querySelectorAll('a[href*="/status/"]');
        var statusUrl = null;
        for (var j = 0; j < links.length; j++) {
            var href = links[j].getAttribute('href');
            var parts = href.split('/').filter(function(p) { return p.length > 0; });
            if (parts.length === 3 && parts[1] === 'status' && !isNaN(parts[2])) {
                statusUrl = href;
                break;
            }
        }
        var statusId = null;
        if (statusUrl) {
            var statusParts = statusUrl.split('/').filter(function(p) { return p.length > 0; });
            statusId = statusParts[statusParts.length - 1].split(/[?#]/)[0];
        }
        var currentId = el.dataset.tweetId ||
                        el.getAttribute('data-item-id') ||
                        el.getAttribute('data-tweet-id') ||
                        statusId;
                        
        if (currentId === tweetId) {
            // Scroll the tweet into the center of the viewport
            el.scrollIntoView({ block: 'center', behavior: 'instant' });
            
            // Query target button
            var btn = el.querySelector('[data-testid="' + buttonName + '"]');
            if (btn) {
                var rect = btn.getBoundingClientRect();
                return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
            }
        }
    }
    return null;
})()
