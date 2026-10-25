//! A counting allocator, for what a search holds (`search::Searched::tree_bytes`): the system allocator, with each
//! thread's live bytes and their peak kept in thread-locals. A binary installs it (`#[global_allocator] static A:
//! Counting = Counting;`, as `cantrip-ai-host` does); without it the counts stay at zero and `installed` says so.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Counting;

static INSTALLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static LIVE: Cell<isize> = const { Cell::new(0) };
    static PEAK: Cell<isize> = const { Cell::new(0) };
}

fn add(n: isize) {
    let _ = LIVE.try_with(|live| {
        let now = live.get() + n;
        live.set(now);
        let _ = PEAK.try_with(|peak| {
            if now > peak.get() {
                peak.set(now);
            }
        });
    });
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            if !INSTALLED.load(Ordering::Relaxed) {
                INSTALLED.store(true, Ordering::Relaxed);
            }
            add(layout.size() as isize);
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            add(layout.size() as isize);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        add(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            add(new_size as isize - layout.size() as isize);
        }
        p
    }
}

/// Whether the counting allocator is the global one (it has counted an allocation).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

/// This thread's live bytes: allocated on it less freed on it.
pub fn live() -> isize {
    LIVE.try_with(Cell::get).unwrap_or(0)
}

/// This thread's peak of live bytes since the last `reset_peak`.
pub fn peak() -> isize {
    PEAK.try_with(Cell::get).unwrap_or(0)
}

/// Starts a new peak at the live bytes now.
pub fn reset_peak() {
    let now = live();
    let _ = PEAK.try_with(|p| p.set(now));
}
