(function() {
    var dialog = document.querySelector('div[role="dialog"]');
    if (!dialog) return null;
    var text = (dialog.textContent || '').toLowerCase();
    if (text.includes('follow') || text.includes('confirm')) {
        return dialog;
    }
    return null;
})()