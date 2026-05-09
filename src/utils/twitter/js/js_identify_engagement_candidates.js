(function() {
    var tweets = [];
    var elements = document.querySelectorAll('article[data-testid="tweet"]');
    for (var i = 0; i < elements.length; i++) {
        var el = elements[i];
        var rect = el.getBoundingClientRect();
        if (rect.height > 0 && rect.width > 0) {
            // Extract tweet text content
            var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
            var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';

            // Find engagement buttons within this tweet element
            var likeBtn = el.querySelector('[data-testid="like"]');
            var retweetBtn = el.querySelector('[data-testid="retweet"]');
            var replyBtn = el.querySelector('[data-testid="reply"]');

            var buttonPositions = {};
            if (likeBtn) {
                var likeRect = likeBtn.getBoundingClientRect();
                if (likeRect.width > 0 && likeRect.height > 0) {
                    buttonPositions.like = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                }
            }
            if (retweetBtn) {
                var retweetRect = retweetBtn.getBoundingClientRect();
                if (retweetRect.width > 0 && retweetRect.height > 0) {
                    buttonPositions.retweet = { x: retweetRect.x + retweetRect.width/2, y: retweetRect.y + retweetRect.height/2 };
                }
            }
            if (replyBtn) {
                var replyRect = replyBtn.getBoundingClientRect();
                if (replyRect.width > 0 && replyRect.height > 0) {
                    buttonPositions.reply = { x: replyRect.x + replyRect.width/2, y: replyRect.y + replyRect.height/2 };
                }
            }

            // Extract the status URL from the time element for reliable diving
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

            // Prefer stable tweet identity from permalink
            var statusId = null;
            if (statusUrl) {
                var statusParts = statusUrl.split('/').filter(function(p) { return p.length > 0; });
                statusId = statusParts[statusParts.length - 1].split(/[?#]/)[0];
            }
            var tweetId = el.dataset.tweetId ||
                          el.getAttribute('data-item-id') ||
                          el.getAttribute('data-tweet-id') ||
                          statusId ||
                          'tweet_' + Math.floor(rect.x) + '_' + Math.floor(rect.y);

            var tweetObj = {
                id: tweetId,
                status_url: statusUrl,
                index: i,
                text: tweetText,
                x: rect.x + rect.width/2,
                y: rect.y + rect.height/2,
                height: rect.height,
                width: rect.width,
                buttons: buttonPositions
            };

            // Extract reply information for smart decision
            var replies = [];
            var replyElements = el.querySelectorAll('[data-testid="tweetReply"]');
            for (var j = 0; j < Math.min(replyElements.length, 3); j++) {
                var replyEl = replyElements[j];
                var authorEl = replyEl.querySelector('[dir="auto"] span:first-child');
                var textEl = replyEl.querySelector('[data-testid="tweetText"]');
                if (authorEl && textEl) {
                    replies.push({
                        author: authorEl.textContent.trim(),
                        text: textEl.textContent.trim()
                    });
                }
            }

            if (replies.length > 0) {
                tweetObj.replies = replies;
            }

            tweets.push(tweetObj);
        }
    }
    return tweets;
})()