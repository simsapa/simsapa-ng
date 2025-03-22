function add_hover_events (el, channel) {
    let href = el.getAttribute('href');
    if (href !== null && href.startsWith('ssp://')) {
        el.addEventListener("mouseover", function(i_el) {
            coords = i_el.target.getBoundingClientRect();

            data = {
                href: href,
                x: coords.x + window.screenX,
                y: coords.y + window.screenY,
                width: coords.width,
                height: coords.height,
            };

            channel.objects.helper.link_mouseover(JSON.stringify(data));
        });

        el.addEventListener("mouseleave", function(i_el) {
            coords = i_el.target.getBoundingClientRect();
            channel.objects.helper.link_mouseleave(href);
        });
    }
}

document.qt_channel = null;

document.addEventListener("DOMContentLoaded", function(event) {
    new QWebChannel(qt.webChannelTransport, function (channel) {
        document.qt_channel = channel;
        var res = document.querySelectorAll("a");
        var arr = [];

        res.forEach((el) => {
            var href = el.getAttribute('href');
            if (href !== null && href.startsWith('ssp://')) {
                arr.push(el);
            }
        });

        arr.forEach(el => add_hover_events(el, channel));
    });

});
