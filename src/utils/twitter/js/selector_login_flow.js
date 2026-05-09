(function() {
    // Login forms
    if (document.querySelector('form[action*="/session"]')) return 'login';
    if (document.querySelector('input[name="session[username_or_email]"]')) return 'login';
    // Phone/email input
    if (document.querySelector('input[type="email"][name*="identifier"]')) return 'login';
    // Onboarding
    if (document.querySelector('form[action*="/i/flow/login"]')) return 'onboarding';
    if (document.querySelector('input[autocomplete="username"]')) return 'onboarding';
    // "Sign in to X" heading/signals
    var h1Elements = document.querySelectorAll('h1');
    for (var i = 0; i < h1Elements.length; i++) {
        var text = (h1Elements[i].textContent || '').toLowerCase();
        if (text.includes('sign in to x') || text.includes('log in to x')) return 'login';
    }
    return null;
})()