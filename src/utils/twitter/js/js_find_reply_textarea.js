(function() {
    var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"]');
    for (var i = 0; i < textboxes.length; i++) {
        var ta = textboxes[i];
        var rect = ta.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) continue;
        ta.focus();
        ta.click();
        return { found: true };
    }
    return { found: false };
})()