//! The engine in a browser (docs/engine-in-the-browser.md): the same JSON requests `cantrip-native-host` serves on stdin
//! (docs/host-protocol.md, `cantrip_host::serve`), served by a WebAssembly module that imports nothing, so any page
//! can instantiate it with no glue but the four exports below.
//!
//! A request is written into memory the module lends (`cantrip_alloc`), answered by `cantrip_call`, which returns the
//! reply's length, and the reply read from `cantrip_reply` before the next call; `cantrip_free` gives the request's
//! memory back. One module holds any number of games, as the host process does. There is no unwinding on this target:
//! where the native host refuses a request the engine failed on, the module traps, and the page reports the failure.

use std::cell::RefCell;

use cantrip_host::serve::Tables;

thread_local! {
    static TABLES: RefCell<Tables> = RefCell::new(Tables::default());
    static REPLY: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Lends `len` bytes of the module's memory, for a request to be written into.
#[unsafe(no_mangle)]
pub extern "C" fn cantrip_alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let at = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    at
}

/// Gives back memory `cantrip_alloc` lent.
///
/// # Safety
/// `at` and `len` are exactly what one `cantrip_alloc` call lent and returned, given back once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cantrip_free(at: *mut u8, len: usize) {
    // SAFETY: the caller gives back a buffer `cantrip_alloc` made with this capacity.
    drop(unsafe { Vec::from_raw_parts(at, 0, len) });
}

/// Answers the request of `len` bytes at `at` (one JSON object, UTF-8): the length of the reply, which `cantrip_reply`
/// points at until the next call.
///
/// # Safety
/// `at` points at `len` initialised bytes the caller wrote, as `cantrip_alloc` lent them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cantrip_call(at: *const u8, len: usize) -> usize {
    // SAFETY: the caller wrote `len` bytes at `at`.
    let bytes = unsafe { std::slice::from_raw_parts(at, len) };
    let reply = match std::str::from_utf8(bytes) {
        Ok(line) => TABLES.with(|tables| tables.borrow_mut().call(line)),
        Err(_) => String::from(r#"{"Rejected":{"Code":"InvalidSetup","Detail":"a request is UTF-8"}}"#),
    };
    REPLY.with(|kept| {
        let mut kept = kept.borrow_mut();
        *kept = reply.into_bytes();
        kept.len()
    })
}

/// Where the last reply is, `cantrip_call`'s length of it long.
#[unsafe(no_mangle)]
pub extern "C" fn cantrip_reply() -> *const u8 {
    REPLY.with(|kept| kept.borrow().as_ptr())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(request: &str) -> String {
        let at = cantrip_alloc(request.len());
        // SAFETY: `at` has room for the request, and is given back after the call.
        unsafe {
            std::ptr::copy_nonoverlapping(request.as_ptr(), at, request.len());
            let len = cantrip_call(at, request.len());
            cantrip_free(at, request.len());
            String::from_utf8(std::slice::from_raw_parts(cantrip_reply(), len).to_vec()).unwrap()
        }
    }

    #[test]
    fn a_request_is_answered_as_the_host_answers_it() {
        let dealt = call(r#"{"Cmd":"NewGame","Seats":[{"Name":"North","Deck":["Island"]},{"Name":"South","Deck":["Swamp"]}],"Seed":1,"MinimumDeckSize":1}"#);
        assert!(dealt.starts_with(r#"{"Ok":{"Decklists""#) || dealt.contains(r#""GameId":1"#), "{dealt}");
        let started = call(r#"{"Cmd":"Start","GameId":1}"#);
        assert!(started.starts_with(r#"{"Ok":"#), "{started}");
        assert!(call("not json").contains("InvalidSetup"));
        assert!(call(r#"{"Cmd":"Start","GameId":9}"#).contains("TableClosed"));
    }
}
