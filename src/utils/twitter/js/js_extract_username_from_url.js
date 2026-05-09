(function() {
    var path = window.location.pathname;
    if (path.startsWith('/')) path = path.substring(1);
    if (path.includes('/')) path = path.split('/')[0];
    return path || null;
})()