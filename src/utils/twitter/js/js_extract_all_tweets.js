(function() {
    var author = 'unknown';
    var text = '';
    var replies = [];
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    var maxArticles = 21;
    for (var i = 0; i < Math.min(articles.length, maxArticles); i++) {
        var el = articles[i];
        var textEl = el.querySelector('[data-testid="tweetText"]');
        var elText = textEl ? textEl.textContent.trim() : '';
        var authorEl = el.querySelector('[dir="auto"]');
        var elAuthor = authorEl ? authorEl.textContent.trim() : 'unknown';

        if (i === 0) {
            author = elAuthor;
            text = elText;
            continue;
        }

        if (!elText || elText.length === 0) continue;

        var likeBtn = el.querySelector('[data-testid="like"]');
        var likePos = null;
        if (likeBtn) {
            var likeRect = likeBtn.getBoundingClientRect();
            if (likeRect.width > 0 && likeRect.height > 0) {
                likePos = { x: likeRect.x + likeRect.width / 2, y: likeRect.y + likeRect.height / 2 };
            }
        }

        var rect = el.getBoundingClientRect();
        var visible = rect.height > 0 && rect.width > 0 && rect.top < window.innerHeight && rect.bottom > 0;

        var statusLink = el.querySelector('a[href*="/status/"]');
        var tweetId = 'unknown';
        if (statusLink) {
            var href = statusLink.getAttribute('href');
            var parts = href.split('/');
            tweetId = parts[parts.length - 1].split('?')[0];
        }

        replies.push({
            id: tweetId,
            text: elText,
            author: elAuthor,
            like_pos: likePos,
            visible: visible,
            y_top: rect.top
        });

        if (replies.length >= 20) break;
    }

    return {
        author: author,
        text: text,
        replies: replies
    };
})()
