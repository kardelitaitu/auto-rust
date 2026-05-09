(function() {
    var replies = [];
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    
    // In a thread view, the first tweet is usually the root tweet.
    // We want to skip it and look at the replies.
    for (var i = 1; i < articles.length; i++) {
        var el = articles[i];
        var rect = el.getBoundingClientRect();
        
        // Only consider visible replies
        if (rect.height > 0 && rect.width > 0 && rect.top < window.innerHeight && rect.bottom > 0) {
            var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
            var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';
            
            // Find Like button
            var likeBtn = el.querySelector('[data-testid="like"]');
            var likePos = null;
            if (likeBtn) {
                var likeRect = likeBtn.getBoundingClientRect();
                if (likeRect.width > 0 && likeRect.height > 0) {
                    likePos = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                }
            }
            
            // Extract author
            var authorEl = el.querySelector('[dir="ltr"] span');
            var author = authorEl ? authorEl.textContent.trim() : 'unknown';
            
            // Extract tweet ID
            var statusLink = el.querySelector('a[href*="/status/"]');
            var tweetId = 'unknown';
            if (statusLink) {
                var href = statusLink.getAttribute('href');
                var parts = href.split('/');
                tweetId = parts[parts.length - 1].split('?')[0];
            }

            if (likePos && tweetText.length > 5) {
                replies.push({
                    id: tweetId,
                    text: tweetText,
                    author: author,
                    like_pos: likePos,
                    y_top: rect.top
                });
            }
        }
        
        if (replies.length >= 8) break; // Scan a reasonable number of visible replies
    }
    return replies;
})()