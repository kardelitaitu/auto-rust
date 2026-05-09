(function() {
    var tweets = [];
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    if (articles.length === 0) {
        articles = document.querySelectorAll('article');
    }
    for (var i = 0; i < articles.length; i++) {
        var el = articles[i];
        var rect = el.getBoundingClientRect();
        var tweetId = el.getAttribute('data-item-id') ||
                      el.getAttribute('data-tweet-id') ||
                      el.getAttribute('data-testid')?.includes('tweet-') ? el.getAttribute('data-testid').replace('tweet-', '') : null;
        tweets.push({
            id: tweetId,
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height
        });
    }
    return tweets;
})()