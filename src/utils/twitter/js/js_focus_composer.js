(function() {
    var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"], [role="textbox"][aria-label="Post text"]');
    for (var i = 0; i < textboxes.length; i++) {
        var textarea = textboxes[i];
        var rect = textarea.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) continue;
        textarea.focus();
        return true;
    }
    return false;
})()
