// A phone at an in-person table (MainPage.InPerson.cs, InPerson/PhoneLink.cs): a short buzz for a tap on the table that
// did nothing, where the browser can vibrate (not iOS), and the relay's welcome kept for this tab alone so a reload takes
// the phone's place back. Defined for every page and called only by a phone's.
globalThis.cantripVibrate = function (milliseconds) {
    try {
        if (typeof navigator.vibrate === "function") {
            navigator.vibrate(milliseconds);
        }
    } catch (e) {
        // Not allowed yet (no gesture on the page so far): the line on screen says it alone.
    }
};

globalThis.cantripSessionSet = function (key, value) {
    try {
        sessionStorage.setItem(key, value);
    } catch (e) {
        // Storage refused (a private window): the ticket takes the place back instead.
    }
};

globalThis.cantripSessionGet = function (key) {
    try {
        return sessionStorage.getItem(key);
    } catch (e) {
        return null;
    }
};
