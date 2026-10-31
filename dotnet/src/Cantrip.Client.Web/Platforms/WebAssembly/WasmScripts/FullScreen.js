// Full screen for in-person play's pages (docs/in-person.md, "Full screen"; InPerson/FullScreen.cs). Defined for every
// page, and nothing in it touches a page unless it was opened with ?inperson=phone or ?inperson=tablet: the remote client
// gains no manifest, no meta tag and no change to its viewport.
//
// - Android, iPad and a desktop browser have the Fullscreen API: the page's button asks for it from its tap, then locks a
//   tablet to landscape and a phone upright where the browser lets it (a phone's browser, once in full screen).
// - An iPhone has no Fullscreen API outside video. Its way to full screen is the page added to the Home Screen, opened
//   as a web app: this file gives the page what iOS reads for that - a manifest of its own (inperson.webmanifest, never
//   Uno's PWA manifest, which would install a service worker), apple-mobile-web-app-capable, a black-translucent status
//   bar - and viewport-fit=cover, so the page lies under the notch and the home indicator and pads itself by
//   env(safe-area-inset-*), read here.
// - A home-screen app has storage of its own, apart from Safari's, and the page's ticket was taken out of the address
//   as it loaded. So the button asks the relay for a re-entry code first, good once, and puts it after the address's #
//   (#reenter=...) - the address the icon opens, whether iOS takes it from the page or from the manifest's start_url,
//   which the manifest leaves to be the page's own. The app's first launch trades it with the relay for a fresh seat
//   ticket, which the app keeps in its own storage and brings on every launch after.
(function () {
    const query = new URLSearchParams(window.location.search);
    const role = (query.get("inperson") || "").toLowerCase();
    const inPerson = role === "phone" || role === "tablet";
    const root = document.documentElement;

    const element = () => document.fullscreenElement || document.webkitFullscreenElement || null;
    const enabled = () => !!(document.fullscreenEnabled || document.webkitFullscreenEnabled)
        && typeof (root.requestFullscreen || root.webkitRequestFullscreen) === "function";
    const matches = (media) => typeof window.matchMedia === "function" && window.matchMedia(media).matches;

    // Opened from the Home Screen (or installed), rather than in a browser tab. A page the Fullscreen API made full
    // screen matches display-mode: fullscreen too, so that alone is not taken for an app.
    const standalone = () => navigator.standalone === true
        || matches("(display-mode: standalone)")
        || (matches("(display-mode: fullscreen)") && !element());

    // An iPad says it is a Mac with a touch screen, and has the Fullscreen API; an iPhone has neither.
    const iPhone = () => /iPhone|iPod/.test(navigator.userAgent || "");

    let changes = 0;
    for (const name of ["fullscreenchange", "webkitfullscreenchange"]) {
        document.addEventListener(name, () => {
            changes++;
            if (!element()) {
                try {
                    screen.orientation.unlock();
                } catch (e) {
                    // Nothing was locked, or the browser cannot.
                }
            }
        });
    }

    // What the page's button is: "exit" while full screen, "enter" where the Fullscreen API is, "homescreen" on an
    // iPhone's browser, "none" anywhere else - an app opened from the Home Screen already has the whole screen. The
    // number of changes since the page loaded follows it, after a comma.
    globalThis.cantripFullScreenState = function () {
        const state = element() ? "exit" : enabled() ? "enter" : standalone() ? "none" : iPhone() ? "homescreen" : "none";
        return state + "," + changes;
    };

    // Asked from the button's tap, which is what lets the page have the screen.
    globalThis.cantripFullScreenEnter = function (landscape) {
        let asked;
        try {
            asked = root.requestFullscreen ? root.requestFullscreen({ navigationUI: "hide" }) : root.webkitRequestFullscreen();
        } catch (e) {
            return;
        }

        Promise.resolve(asked).then(() => {
            try {
                const orientation = screen.orientation;
                if (orientation && typeof orientation.lock === "function") {
                    orientation.lock(landscape ? "landscape" : "portrait").catch(() => { });
                }
            } catch (e) {
                // No lock here (a desktop, iPadOS): the page lays itself out for whichever way it is held.
            }
        }).catch(() => { });
    };

    globalThis.cantripFullScreenExit = function () {
        try {
            const left = document.exitFullscreen ? document.exitFullscreen() : document.webkitExitFullscreen();
            Promise.resolve(left).catch(() => { });
        } catch (e) {
            // Not full screen after all.
        }
    };

    // The notch, the status bar and the home indicator: how far each edge of the page is covered, "top,right,bottom,left"
    // in CSS pixels, read from env(safe-area-inset-*) through an element padded by them. Zero everywhere but a phone or
    // tablet that has them, with viewport-fit=cover.
    let probe = null;
    globalThis.cantripSafeArea = function () {
        if (!inPerson || !document.body) {
            return "0,0,0,0";
        }

        if (!probe) {
            probe = document.createElement("div");
            probe.setAttribute("aria-hidden", "true");
            probe.style.cssText = "position:fixed;left:0;top:0;width:0;height:0;visibility:hidden;pointer-events:none;"
                + "padding:env(safe-area-inset-top,0px) env(safe-area-inset-right,0px) env(safe-area-inset-bottom,0px) env(safe-area-inset-left,0px);";
            document.body.appendChild(probe);
        }

        const style = window.getComputedStyle(probe);
        return [style.paddingTop, style.paddingRight, style.paddingBottom, style.paddingLeft].map((each) => parseFloat(each) || 0).join(",");
    };

    // Puts a phone's re-entry code after the address's # for the icon iOS is about to make of the page, and has the
    // manifest read again, so a start_url iOS takes from it is this address too.
    globalThis.cantripOfferHomeScreen = function (code) {
        const url = new URL(window.location.href);
        const fragment = new URLSearchParams(url.hash.slice(1));
        fragment.set("reenter", code);
        url.hash = fragment.toString();
        window.history.replaceState(window.history.state, "", url.toString());

        const manifest = document.querySelector('link[rel="manifest"]');
        if (manifest) {
            const again = manifest.cloneNode();
            manifest.remove();
            document.head.appendChild(again);
        }
    };

    // A home-screen app's phone keeps the seat ticket the relay traded its re-entry code for, then loads again to take
    // its seat with it.
    const ticketKey = "cantrip-inperson-ticket:" + (query.get("server") || "");
    globalThis.cantripKeepTicket = function (ticket) {
        try {
            localStorage.setItem(ticketKey, ticket);
        } catch (e) {
            return;
        }

        // The spent code is left in the address: the ticket kept is preferred to it from now on.
        window.location.reload();
    };

    if (!inPerson) {
        return;
    }

    const head = document.head;
    const meta = (name, content) => {
        let tag = head.querySelector(`meta[name="${name}"]`);
        if (!tag) {
            tag = document.createElement("meta");
            tag.name = name;
            head.appendChild(tag);
        }

        tag.content = content;
    };
    const link = (rel, href) => {
        if (!head.querySelector(`link[rel="${rel}"]`)) {
            const tag = document.createElement("link");
            tag.rel = rel;
            tag.href = href;
            head.appendChild(tag);
        }
    };

    meta("apple-mobile-web-app-capable", "yes");
    meta("mobile-web-app-capable", "yes");
    meta("apple-mobile-web-app-status-bar-style", "black-translucent");
    meta("apple-mobile-web-app-title", "Cantrip");
    link("apple-touch-icon", "inperson-icon-180.png");
    link("manifest", "inperson.webmanifest");

    const viewport = head.querySelector('meta[name="viewport"]');
    if (viewport && !/viewport-fit/.test(viewport.content)) {
        viewport.content = viewport.content + ", viewport-fit=cover";
    }

    // A phone opened from the Home Screen: the ticket it kept, or on its first launch the re-entry code in the address
    // it was added with (spent once traded, so the ticket kept is preferred on every launch after). A code in a browser
    // tab's address is left there, for the icon still to be made of it.
    if (role === "phone" && standalone()) {
        const fragment = new URLSearchParams(window.location.hash.slice(1));
        const arrived = globalThis.cantripTicket || fragment.get("ticket");
        let kept = null;
        try {
            if (arrived) {
                localStorage.setItem(ticketKey, arrived);
            } else {
                kept = localStorage.getItem(ticketKey);
            }
        } catch (e) {
            // Storage refused: the re-entry code is all there is.
        }

        if (kept) {
            globalThis.cantripTicket = kept;
        } else if (!arrived && fragment.get("reenter")) {
            globalThis.cantripReentry = fragment.get("reenter");
        }
    }
})();
