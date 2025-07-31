function show_transient_message(text: string, msg_div_id: string): void {
    const div = document.createElement('div');
    div.className = 'message';

    const content = document.createElement('div');
    content.className = 'msg-content';
    content.textContent = text;
    div.appendChild(content);

    let el = document.getElementById(msg_div_id);
    if (!el) {
        console.error("Cannot find: transient-messages");
        return;
    }

    el.appendChild(div);

    div.style.transition = 'opacity 1.5s ease-in-out';
    div.style.opacity = '1';

    // After 3 seconds, start fading out
    setTimeout(() => {
        div.style.opacity = '0';
    }, 1000);

    // After the transition ends, remove the div from the DOM
    div.addEventListener('transitionend', () => {
        div.remove();
    });
}

export {
    show_transient_message,
}
