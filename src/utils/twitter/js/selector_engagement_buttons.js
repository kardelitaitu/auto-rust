(function() {
    var result = {
        like: null,
        retweet: null,
        reply: null
    };
    var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
    for (var i = 0; i < buttons.length; i++) {
        var el = buttons[i];
        var testId = (el.getAttribute('data-testid') || '').toLowerCase();
        var rect = el.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) continue;
        var pos = { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
        if (testId.includes('like') && !testId.includes('unlike')) {
            result.like = pos;
        } else if (testId.includes('retweet') && !testId.includes('unretweet')) {
            result.retweet = pos;
        } else if (testId.includes('reply') || testId.includes('comment')) {
            result.reply = pos;
        }
    }
    return result;
})()