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

/// The signal received, set by the handler and polled through [`interrupted`].
static SIGNAL: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

const STDERR: i32 = 2;
const SIGINT_MESSAGE: &str = "received SIGINT, shutting down\n";
const SIGTERM_MESSAGE: &str = "received SIGTERM, shutting down\n";

unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
    fn write(descriptor: i32, buffer: *const u8, count: usize) -> isize;
    fn _exit(code: i32) -> !;
}

/// Say so on stderr, note the interrupt for the polling loop, and get out hard
/// when the user insists with a second one.
extern "C" fn on_interrupt(number: i32) {
    if SIGNAL.swap(number, std::sync::atomic::Ordering::Relaxed) != 0 {
        unsafe { _exit(INTERRUPT_EXIT_CODE) }
    }

    let message = if number == SIGINT {
        SIGINT_MESSAGE
    } else {
        SIGTERM_MESSAGE
    };
    unsafe { write(STDERR, message.as_ptr(), message.len()) };
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
    SIGNAL.load(std::sync::atomic::Ordering::Relaxed) != 0
}
