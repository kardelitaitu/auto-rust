(function() {
    var results = {
        feed_visible: false,
        tweets_found: false,
        engagement_buttons: false,
        follow_button: false
    };

    // Check feed visibility
    if (document.querySelector('[data-testid="primaryColumn"]') ||
        document.querySelector('main[role="main"]') ||
        document.querySelector('article[data-testid="tweet"]')) {
        results.feed_visible = true;
    }

    // Check for tweets
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    if (articles.length > 0 || document.querySelectorAll('article').length > 0) {
        results.tweets_found = true;
    }

    // Check for engagement buttons
    var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
    for (var i = 0; i < buttons.length; i++) {
        var testId = (buttons[i].getAttribute('data-testid') || '').toLowerCase();
        if (testId.includes('like') || testId.includes('retweet') || testId.includes('reply')) {
            results.engagement_buttons = true;
            break;
        }
    }

    // Check for follow button
    var allButtons = document.querySelectorAll('[role="button"]');
    for (var i = 0; i < allButtons.length; i++) {
        var label = (allButtons[i].getAttribute('aria-label') || '').toLowerCase();
        if (label.includes('follow')) {
            results.follow_button = true;
            break;
        }
    }

    return results;
})()