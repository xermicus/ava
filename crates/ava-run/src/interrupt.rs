//! Signal handling so a run outlives its terminal and dies clean otherwise.
//!
//! `SIGHUP` is ignored: a dropped ssh connection or closed terminal must not
//! cut a run short. `SIGINT` and `SIGTERM` flip a flag the run loop polls, so
//! the containers are torn down before the process ends.

/// The exit code convention for death by signal.
const INTERRUPT_EXIT_CODE: i32 = 130;

/// libc's `SIG_IGN` disposition.
const SIG_IGN: usize = 1;

/// Signal numbers, identical on every POSIX platform.
const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;

/// Set by the handler, polled through [`interrupted`].
static INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
    fn _exit(code: i32) -> !;
}

/// Note the interrupt for the polling loop, and get out hard when the user
/// insists with a second one.
extern "C" fn on_interrupt(_: i32) {
    if INTERRUPTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        unsafe { _exit(INTERRUPT_EXIT_CODE) }
    }
}

/// Install the signal dispositions.
pub fn install() {
    unsafe {
        signal(SIGHUP, SIG_IGN);
        signal(SIGINT, on_interrupt as *const () as usize);
        signal(SIGTERM, on_interrupt as *const () as usize);
    }
}

/// Whether the process was asked to stop.
pub fn interrupted() -> bool {
    INTERRUPTED.load(std::sync::atomic::Ordering::Relaxed)
}
