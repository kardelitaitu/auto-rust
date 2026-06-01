(function() {
    var authorEl = document.querySelector('article[data-testid="tweet"] [dir="auto"]');
    var author = authorEl ? authorEl.textContent.trim() : 'unknown';

    var tweetEl = document.querySelector('[data-testid="tweetText"]');
    var text = tweetEl ? tweetEl.textContent.trim() : '';

    var replies = [];
    var replyEls = document.querySelectorAll('article[data-testid="tweet"]');
    for (var i = 1; i < Math.min(replyEls.length, 21); i++) {
        var reply = replyEls[i];
        var replyAuthorEl = reply.querySelector('[dir="auto"]');
        var replyTextEl = reply.querySelector('[data-testid="tweetText"]');
        var replyAuthor = replyAuthorEl ? replyAuthorEl.textContent.trim() : 'unknown';
        var replyText = replyTextEl ? replyTextEl.textContent.trim() : '';
        if (replyText && replyText.length > 0) {
            replies.push({ author: replyAuthor, text: replyText });
        }
    }

    return {
        author: author,
        text: text,
        replies: replies.map(function(r) { return [r.author, r.text]; })
    };
})()
