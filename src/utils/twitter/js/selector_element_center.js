(function() {
    var el = document.querySelector("{SELECTOR}");
    if (!el) return null;
    var rect = el.getBoundingClientRect();
    return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
})()