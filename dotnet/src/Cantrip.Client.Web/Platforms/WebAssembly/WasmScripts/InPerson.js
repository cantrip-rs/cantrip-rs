// A tablet's device token at an in-person table (docs/in-person.md) is a bearer credential for its board, so it is taken
// out of the address bar as the page loads - before it can be copied, bookmarked or kept in history - and held here for
// InPersonConfig to read once, as Ticket.js does a seat ticket. It comes after the hash (#device=...), which no request
// and no log ever holds. Anything else after the hash is left where it is.
(function () {
    const url = new URL(window.location.href);
    const fragment = new URLSearchParams(url.hash.slice(1));
    const device = fragment.get("device");
    if (device) {
        globalThis.cantripDevice = device;
        fragment.delete("device");
        const rest = fragment.toString();
        url.hash = rest.length > 0 ? rest : "";
        window.history.replaceState(window.history.state, "", url.toString());
    }
})();

// Under ?test, a tablet with no device token is fed by the test rather than a relay (InPerson/TabletTestState.cs): each
// message handed in, base64 MessagePack, waits here until the page takes them all.
(function () {
    if (!new URLSearchParams(window.location.search).has("test")) {
        return;
    }

    const queue = [];
    globalThis.cantripTabletFeed = (message) => queue.push(message);
    globalThis.cantripTakeTabletFeed = () => {
        const taken = queue.join("\n");
        queue.length = 0;
        return taken;
    };
})();
