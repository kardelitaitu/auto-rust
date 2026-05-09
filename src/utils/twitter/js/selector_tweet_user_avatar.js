(function() {
    // Try multiple selector strategies for tweet user avatar
    var avatar = document.querySelector('[data-testid="Tweet-User-Avatar"]') ||
                document.querySelector('article img[src*="/profile_images"]') ||
                document.querySelector('[role="article"] img');
    if (avatar) {
        var rect = avatar.getBoundingClientRect();
        return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
    }
    return null;
})()