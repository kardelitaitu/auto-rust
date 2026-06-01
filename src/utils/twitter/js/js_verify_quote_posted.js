(function() {
    var textarea = document.querySelector('[data-testid="tweetTextarea_0"]') ||
                   document.querySelector('[role="textbox"]');
    if (!textarea) return { posted: true, reason: 'composer closed' };
    var text = textarea.textContent || textarea.value || '';
    if (text.trim() === '') return { posted: true, reason: 'composer cleared' };
    return { posted: false, reason: 'composer still contains text' };
})()
