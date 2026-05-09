(function() {
    var btn = document.querySelector('button[data-testid="retweetConfirm"]');
    if (!btn) return null;
    var rect = btn.getBoundingClientRect();
    return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
})()