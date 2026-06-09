(function() {
    var textarea = document.querySelector('[data-testid="tweetTextarea_0"]') ||
                   document.querySelector('[role="textbox"]');
    if (!textarea) return { posted: true, reason: 'composer closed' };
    var text = textarea.textContent || textarea.value || '';
    if (text.trim() !== '') return { posted: false, reason: 'composer still contains text' };

    // Composer is cleared — now check secondary signals
    var reasons = [];

    // Signal 1: URL changed (navigated to the posted quote)
    if (window.location.pathname.includes('/status/')) {
        reasons.push('url_has_status');
    }

    // Signal 2: discoverable new tweet (composer was just open, now gone)
    var articles = document.querySelectorAll('article[data-testid="tweet"]');
    if (articles.length > 0) {
        reasons.push('tweets_visible');
    }

    // Signal 3: composer dialog no longer open
    var dialog = document.querySelector('div[role="dialog"]');
    if (!dialog) {
        reasons.push('no_dialog');
    }

    if (reasons.length >= 2) {
        return { posted: true, reason: reasons.join('+') };
    }
    return { posted: true, reason: 'composer cleared' };
})()
