//! The crackme game: in the first turn every seat writes a crackme with the
//! keygen that unlocks it, in the second every seat writes one keygen for the
//! crackmes of the others.
//!
//! A keygen turns a number into a key: the same key for the same number, a
//! different key for a different number. A crackme gets a number and a key
//! and exits 0 when the key is the one its keygen makes for that number, 1
//! for anything else. Since the crackme checks the key against the number,
//! keys found for other numbers open nothing, and a solver has to reproduce
//! the function. The defence is verified by running a sample of random numbers
//! through the author's keygen into the crackme, and by pairs the crackme
//! must refuse. The attack is verified by running a sample per crackme through
//! the attacker's keygen into that crackme, seeded as an input of the turn,
//! and records the crackmes it cracked. Both binaries run confined to the
//! system directories and themselves, so neither can read or run the other.

const GAME_NAME: &str = "crackme";

/// The crackme, an ELF started with a number and a key as its arguments.
const CRACKME: &str = "crackme";

/// The keygen, an ELF started with a number as its only argument, printing the key.
const KEYGEN: &str = "keygen";

/// How many random numbers a verification runs through a keygen.
const SAMPLE: usize = 16;

/// How many of the sampled numbers are tried with keys the crackme must refuse.
const ALTERED: usize = 4;

const SIZE_LIMIT: u64 = 1 << 20;

/// The characters a key has at least, so the space a check can spread over
/// is out of reach of a search, and at most.
const KEY_MINIMUM: usize = 20;
const KEY_LIMIT: usize = 256;
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
/// The seconds a binary has to answer, ample under emulation too.
const RUN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const WAIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
const OUTPUT_LIMIT: u64 = 4096;

/// What a binary of the task may reach on the filesystem besides itself: the
/// directories a dynamically linked binary starts from.
const SYSTEM_DIRECTORIES: [&str; 6] = ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];

/// Keys every crackme must refuse for any number, besides the altered keys
/// and the keys of other numbers.
const REFUSED_KEYS: [&str; 2] = ["", "password"];

/// The crackme game: a crackme with its keygen in the first turn, one keygen
/// for the crackmes of the other seats in the second.
pub struct Crackme;

/// The turns: defending with a crackme, then attacking the crackmes of the others.
const TURNS: [crate::Turn; 2] = [
    crate::Turn {
        task: "defend",
        entry: CRACKME,
    },
    crate::Turn {
        task: "attack",
        entry: KEYGEN,
    },
];
const DEFEND: usize = 0;
const ATTACK: usize = 1;

/// The name the crackme of `seat` is seeded under for the attackers, the seat
/// counted from one the way the tournament page counts.
fn crackme_of(seat: usize) -> String {
    format!("{CRACKME}.{}", seat + 1)
}

/// The seat whose crackme is seeded as `name`, if it is one.
fn seat_of(name: &str) -> Option<usize> {
    name.strip_prefix(CRACKME)?
        .strip_prefix('.')?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
}

impl crate::Game for Crackme {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn turns(&self) -> &[crate::Turn] {
        &TURNS
    }

    /// The attackers get every opponent's crackme, named by its seat.
    fn inputs(&self, turn: usize, opponents: &[usize]) -> Vec<crate::Input> {
        if turn != ATTACK {
            return Vec::new();
        }

        opponents
            .iter()
            .map(|&seat| crate::Input {
                seat,
                turn: DEFEND,
                name: crackme_of(seat),
            })
            .collect()
    }

    fn verify(
        &self,
        turn: usize,
        submission: &std::path::Path,
        inputs: &std::path::Path,
    ) -> std::io::Result<ava_wire::Verdict> {
        match turn {
            DEFEND => verify_defence(submission),
            ATTACK => verify_attack(submission, inputs),
            _ => Err(std::io::Error::other(format!(
                "{GAME_NAME} has {} turns",
                TURNS.len()
            ))),
        }
    }

    /// Two rounds, one per direction of attack: a seat wins the round it
    /// attacks in when a push of its attack cracked the other's crackme, and
    /// by forfeit when the other left none.
    fn outcome(
        &self,
        first: (usize, &[crate::Played]),
        second: (usize, &[crate::Played]),
    ) -> Option<crate::Outcome> {
        let defended =
            |played: &[crate::Played]| played.get(DEFEND).is_some_and(|turn| turn.entry.is_some());
        if !defended(first.1) && !defended(second.1) {
            return Some(crate::forfeit(first.0, false, second.0, false));
        }

        let mut tally = ava_wire::Tally::default();
        let mut reasons = Vec::new();
        for (attacker, defender, won) in [(first, second, true), (second, first, false)] {
            let cracked = if defended(defender.1) {
                cracked(attacker.1, defender.0)
            } else {
                reasons.push(format!("seat {} left no {CRACKME}", defender.0 + 1));
                true
            };
            if cracked == won {
                tally.won += 1;
            } else {
                tally.lost += 1;
            }
        }

        Some(crate::Outcome {
            tally,
            reason: (!reasons.is_empty()).then(|| reasons.join(", ")),
        })
    }
}

/// Whether a push of the attack of `played` cracked the crackme of `defender`.
fn cracked(played: &[crate::Played], defender: usize) -> bool {
    let name = crackme_of(defender);
    played.get(ATTACK).is_some_and(|turn| {
        turn.attempts
            .iter()
            .any(|attempt| attempt.verdict.defeated.contains(&name))
    })
}

/// Verify that the crackme accepts the key the keygen makes for a number,
/// and refuses altered keys, the keys of other numbers and fixed wrong keys.
fn verify_defence(submission: &std::path::Path) -> std::io::Result<ava_wire::Verdict> {
    let crackme = submission.join(CRACKME);
    let keygen = submission.join(KEYGEN);
    for (binary, name) in [(&crackme, CRACKME), (&keygen, KEYGEN)] {
        if let Some(reason) = unfit_binary(binary, name)? {
            return Ok(crate::failed(reason));
        }
    }

    let numbers = sample();
    let keys = match generated(&keygen, &[], &numbers)? {
        Ok(keys) => keys,
        Err(reason) => return Ok(crate::failed(reason)),
    };

    for (number, key) in numbers.iter().zip(&keys) {
        if let Some(reason) = refused(&crackme, *number, key)? {
            return Ok(crate::failed(format!(
                "{CRACKME} refuses the key of {number} from its own {KEYGEN}: {reason}"
            )));
        }
    }

    for (index, (number, key)) in numbers.iter().zip(&keys).enumerate().take(ALTERED) {
        let other = &keys[(index + 1) % keys.len()];
        for wrong in REFUSED_KEYS
            .iter()
            .map(|wrong| wrong.to_string())
            .chain([altered(key), other.clone()])
        {
            if refused(&crackme, *number, &wrong)?.is_none() {
                return Ok(crate::failed(format!(
                    "{CRACKME} accepts `{wrong}` for {number}, which its {KEYGEN} never made for it"
                )));
            }
        }
    }

    log::info!(
        "{CRACKME} accepts the {SAMPLE} keys of its {KEYGEN} for their numbers and refuses the wrong ones"
    );

    Ok(ava_wire::Verdict::passed())
}

/// Verify the keygen against every crackme seeded as an input: a crackme is
/// cracked when it accepts the key the keygen makes for each sampled number,
/// and the push passes when at least one is.
fn verify_attack(
    submission: &std::path::Path,
    inputs: &std::path::Path,
) -> std::io::Result<ava_wire::Verdict> {
    let keygen = submission.join(KEYGEN);
    if let Some(reason) = unfit_binary(&keygen, KEYGEN)? {
        return Ok(crate::failed(reason));
    }

    let mut crackmes: Vec<(usize, std::path::PathBuf)> = match std::fs::read_dir(inputs) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let seat = seat_of(entry.file_name().to_str()?)?;
                Some((seat, entry.path()))
            })
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    crackmes.sort_by_key(|(seat, _)| *seat);
    if crackmes.is_empty() {
        return Ok(crate::failed(format!(
            "no {CRACKME} to attack, no other seat left one"
        )));
    }

    let mut defeated = Vec::new();
    let mut reasons = Vec::new();
    for (seat, crackme) in crackmes {
        let name = crackme_of(seat);
        let numbers = sample();
        let seat_argument = (seat + 1).to_string();
        let keys = match generated(&keygen, &[&seat_argument], &numbers)? {
            Ok(keys) => keys,
            Err(reason) => {
                reasons.push(format!("{name}: {reason}"));
                continue;
            }
        };

        let mut refusal = None;
        for (number, key) in numbers.iter().zip(&keys) {
            if let Some(reason) = refused(&crackme, *number, key)? {
                refusal = Some(format!("{name} refuses the key of {number}: {reason}"));
                break;
            }
        }
        match refusal {
            Some(reason) => reasons.push(reason),
            None => defeated.push(name),
        }
    }

    log::info!(
        "the {KEYGEN} cracked {} of the crackmes: {}",
        defeated.len(),
        defeated.join(" ")
    );
    for reason in &reasons {
        log::info!("{reason}");
    }

    if defeated.is_empty() {
        return Ok(crate::failed(reasons.join("; ")));
    }

    Ok(ava_wire::Verdict::defeated(defeated))
}

/// Random numbers for one verification, distinct from each other, so a
/// keygen cannot be a table and a crackme cannot be a list.
fn sample() -> Vec<u64> {
    let state = std::collections::hash_map::RandomState::new();
    let mut numbers: Vec<u64> = Vec::new();

    for index in 0.. {
        if numbers.len() == SAMPLE {
            break;
        }
        let mut hasher = std::hash::BuildHasher::build_hasher(&state);
        std::hash::Hasher::write_usize(&mut hasher, index);
        let number = std::hash::Hasher::finish(&hasher);
        if !numbers.contains(&number) {
            numbers.push(number);
        }
    }

    numbers
}

/// The keys `keygen` makes for `numbers`, or why it is not a keygen: it has
/// to answer every number, with one key, the same one when asked again, and
/// with different keys for different numbers.
fn generated(
    keygen: &std::path::Path,
    prefix: &[&str],
    numbers: &[u64],
) -> std::io::Result<Result<Vec<String>, String>> {
    let mut keys: Vec<String> = Vec::new();

    for number in numbers {
        let number_argument = number.to_string();
        let arguments: Vec<&str> = prefix
            .iter()
            .copied()
            .chain([number_argument.as_str()])
            .collect();
        let first = match key_of(keygen, &arguments)? {
            Ok(key) => key,
            Err(reason) => return Ok(Err(reason)),
        };
        let second = match key_of(keygen, &arguments)? {
            Ok(key) => key,
            Err(reason) => return Ok(Err(reason)),
        };
        if first != second {
            return Ok(Err(format!(
                "{KEYGEN} makes different keys for {number} when asked twice"
            )));
        }
        if let Some(earlier) = keys.iter().position(|key| *key == first) {
            return Ok(Err(format!(
                "{KEYGEN} makes the same key for {} and {number}",
                numbers[earlier]
            )));
        }
        keys.push(first);
    }

    Ok(Ok(keys))
}

/// The key `keygen` prints for `arguments`, or why what it printed is not a key.
fn key_of(keygen: &std::path::Path, arguments: &[&str]) -> std::io::Result<Result<String, String>> {
    let argument = arguments.join(" ");
    let Some((status, output)) = run_with_timeout(keygen, arguments)? else {
        return Ok(Err(format!(
            "{KEYGEN} gave no answer within {RUN_TIMEOUT:?} for {argument}"
        )));
    };
    if !status.success() {
        return Ok(Err(format!("{KEYGEN} exited with {status} for {argument}")));
    }

    let printed = String::from_utf8_lossy(&output);
    let key = printed.trim();
    if key.is_empty() {
        return Ok(Err(format!("{KEYGEN} printed no key for {argument}")));
    }
    if key.len() < KEY_MINIMUM || key.len() > KEY_LIMIT {
        return Ok(Err(format!(
            "{KEYGEN} printed {} characters for {argument}, a key has {KEY_MINIMUM} to {KEY_LIMIT}",
            key.len()
        )));
    }
    if !key.chars().all(|character| character.is_ascii_graphic()) {
        return Ok(Err(format!(
            "{KEYGEN} printed more than one line of printable ASCII without whitespace for {argument}"
        )));
    }

    Ok(Ok(key.to_string()))
}

/// `key` with its first character changed, a key the crackme must refuse.
fn altered(key: &str) -> String {
    let mut characters: Vec<char> = key.chars().collect();
    characters[0] = if characters[0] == 'a' { 'b' } else { 'a' };
    characters.into_iter().collect()
}

/// Why `binary` cannot be the `name` binary of the task, if it cannot.
fn unfit_binary(binary: &std::path::Path, name: &str) -> std::io::Result<Option<String>> {
    let contents = match crate::read_at_most(binary, SIZE_LIMIT) {
        Ok(Some(contents)) => contents,
        Ok(None) => {
            return Ok(Some(format!(
                "{name} is over {SIZE_LIMIT} bytes, the task allows at most that"
            )));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Some(format!("no {name} in the submission")));
        }
        Err(error) => return Ok(Some(format!("{name} cannot be read: {error}"))),
    };

    if !contents.starts_with(&ELF_MAGIC) {
        return Ok(Some(format!("{name} is not an ELF")));
    }

    Ok(None)
}

/// Why `crackme` refuses `key` for `number`, or nothing when it accepts the
/// pair by exiting 0.
fn refused(crackme: &std::path::Path, number: u64, key: &str) -> std::io::Result<Option<String>> {
    let Some((status, _)) = run_with_timeout(crackme, &[&number.to_string(), key])? else {
        return Ok(Some(format!(
            "no answer within {RUN_TIMEOUT:?} for `{key}`"
        )));
    };

    if status.success() {
        Ok(None)
    } else {
        Ok(Some(format!("{status} for `{key}`")))
    }
}

/// Run `binary` on `arguments`, confined to the system directories and itself,
/// and return its exit status and standard output, or `None` after killing it
/// on timeout.
///
/// Both binaries of the task read nothing but their arguments. In the solve
/// container the keygen sits next to the crackme it has to reproduce, so
/// unconfined it could run the crackme as an oracle, and a crackme could read
/// the keygen it is asked to accept and refuse every one but its author's.
fn run_with_timeout(
    binary: &std::path::Path,
    arguments: &[&str],
) -> std::io::Result<Option<(std::process::ExitStatus, Vec<u8>)>> {
    let mut command = crate::binary_command(binary);
    command
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    confine(&mut command, binary)?;

    let mut child = command.spawn().map_err(|error| {
        std::io::Error::other(format!("{} cannot be started: {error}", binary.display()))
    })?;

    let stdout = child.stdout.take().expect("stdout was requested piped");
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut capped = std::io::Read::take(stdout, OUTPUT_LIMIT);
        std::io::Read::read_to_end(&mut capped, &mut output).map(|_| output)
    });

    let deadline = std::time::Instant::now() + RUN_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        std::thread::sleep(WAIT_INTERVAL);
    };

    let output = reader.join().expect("the reader thread does not panic")?;
    Ok(Some((status, output)))
}

/// Let `command` reach nothing on the filesystem but the system directories
/// and `binary`, through a Landlock ruleset the child applies to itself
/// before it executes.
fn confine(command: &mut std::process::Command, binary: &std::path::Path) -> std::io::Result<()> {
    let ruleset = landlock::Ruleset::new()?;
    for directory in SYSTEM_DIRECTORIES {
        let directory = std::path::Path::new(directory);
        if directory.is_dir() {
            ruleset.allow(directory, landlock::READ_AND_EXECUTE)?;
        }
    }
    ruleset.allow(binary, landlock::EXECUTE_FILE)?;

    // The ruleset is built here, so the child only applies it: two system
    // calls and no allocation between fork and exec.
    unsafe {
        std::os::unix::process::CommandExt::pre_exec(command, move || ruleset.restrict_self());
    }

    Ok(())
}

/// Landlock, the Linux facility letting an unprivileged process restrict the
/// filesystem it may reach.
#[cfg(target_os = "linux")]
mod landlock {
    use std::os::fd::AsRawFd;

    const SYS_CREATE_RULESET: std::ffi::c_long = 444;
    const SYS_ADD_RULE: std::ffi::c_long = 445;
    const SYS_RESTRICT_SELF: std::ffi::c_long = 446;
    const PR_SET_NO_NEW_PRIVS: std::ffi::c_int = 38;
    const CREATE_RULESET_VERSION: std::ffi::c_uint = 1;
    const RULE_PATH_BENEATH: std::ffi::c_uint = 1;
    const NO_FLAGS: std::ffi::c_uint = 0;

    const ACCESS_FS_EXECUTE: u64 = 1 << 0;
    const ACCESS_FS_READ_FILE: u64 = 1 << 2;
    const ACCESS_FS_READ_DIR: u64 = 1 << 3;

    /// Every access right of the first ABI.
    const ACCESS_FS_ABI_1: u64 = (1 << 13) - 1;

    /// The rights later ABIs added, by the ABI that added each, handled once
    /// the kernel knows them so they are denied like the rest.
    const ACCESS_FS_LATER: [(std::ffi::c_long, u64); 3] =
        [(2, 1 << 13), (3, 1 << 14), (5, 1 << 15)];

    /// Reading and executing beneath a directory.
    pub(super) const READ_AND_EXECUTE: u64 =
        ACCESS_FS_READ_FILE | ACCESS_FS_READ_DIR | ACCESS_FS_EXECUTE;

    /// Reading and executing one file.
    pub(super) const EXECUTE_FILE: u64 = ACCESS_FS_READ_FILE | ACCESS_FS_EXECUTE;

    #[repr(C)]
    struct RulesetAttr {
        handled_access_fs: u64,
    }

    #[repr(C, packed)]
    struct PathBeneathAttr {
        allowed_access: u64,
        parent_fd: std::ffi::c_int,
    }

    unsafe extern "C" {
        fn syscall(number: std::ffi::c_long, ...) -> std::ffi::c_long;
        fn prctl(option: std::ffi::c_int, ...) -> std::ffi::c_int;
    }

    /// A ruleset denying every handled access unless a rule allows it.
    pub(super) struct Ruleset(std::os::fd::OwnedFd);

    impl Ruleset {
        pub(super) fn new() -> std::io::Result<Self> {
            let abi = unsafe {
                syscall(
                    SYS_CREATE_RULESET,
                    std::ptr::null::<RulesetAttr>(),
                    0usize,
                    CREATE_RULESET_VERSION,
                )
            };
            if abi < 0 {
                return Err(std::io::Error::other(format!(
                    "the kernel has no landlock, the binaries of the task cannot be confined: {}",
                    std::io::Error::last_os_error()
                )));
            }

            let mut handled = ACCESS_FS_ABI_1;
            for (since, right) in ACCESS_FS_LATER {
                if abi >= since {
                    handled |= right;
                }
            }
            let attr = RulesetAttr {
                handled_access_fs: handled,
            };
            let fd = unsafe {
                syscall(
                    SYS_CREATE_RULESET,
                    &attr as *const RulesetAttr,
                    std::mem::size_of::<RulesetAttr>(),
                    NO_FLAGS,
                )
            };
            if fd < 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self(unsafe {
                std::os::fd::FromRawFd::from_raw_fd(fd as std::ffi::c_int)
            }))
        }

        /// Allow `access` beneath `path`, a directory or a single file.
        pub(super) fn allow(&self, path: &std::path::Path, access: u64) -> std::io::Result<()> {
            let opened = std::fs::File::open(path)?;
            let rule = PathBeneathAttr {
                allowed_access: access,
                parent_fd: opened.as_raw_fd(),
            };
            let added = unsafe {
                syscall(
                    SYS_ADD_RULE,
                    self.0.as_raw_fd(),
                    RULE_PATH_BENEATH,
                    &rule as *const PathBeneathAttr,
                    NO_FLAGS,
                )
            };
            if added < 0 {
                return Err(std::io::Error::other(format!(
                    "{}: {}",
                    path.display(),
                    std::io::Error::last_os_error()
                )));
            }

            Ok(())
        }

        /// Restrict the calling process for good.
        pub(super) fn restrict_self(&self) -> std::io::Result<()> {
            if unsafe { prctl(PR_SET_NO_NEW_PRIVS, 1usize, 0usize, 0usize, 0usize) } != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if unsafe { syscall(SYS_RESTRICT_SELF, self.0.as_raw_fd(), NO_FLAGS) } != 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(())
        }
    }
}

/// The verifier only runs in the scoring container, which is linux.
#[cfg(not(target_os = "linux"))]
mod landlock {
    pub(super) const READ_AND_EXECUTE: u64 = 0;
    pub(super) const EXECUTE_FILE: u64 = 0;

    pub(super) struct Ruleset;

    impl Ruleset {
        pub(super) fn new() -> std::io::Result<Self> {
            Err(std::io::Error::other(
                "the binaries of the task can only be confined on linux",
            ))
        }

        pub(super) fn allow(&self, _path: &std::path::Path, _access: u64) -> std::io::Result<()> {
            Ok(())
        }

        pub(super) fn restrict_self(&self) -> std::io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Game;

    fn played(entry: bool, defeated: &[&str]) -> crate::Played {
        crate::Played {
            entry: entry.then(|| crate::Kept {
                path: std::path::PathBuf::from(super::CRACKME),
                points: None,
            }),
            attempts: defeated
                .iter()
                .map(|name| ava_wire::Attempt {
                    seconds: 1,
                    verdict: ava_wire::Verdict::defeated(vec![name.to_string()]),
                })
                .collect(),
        }
    }

    #[test]
    fn the_attackers_get_every_other_crackme_by_seat() {
        let inputs = super::Crackme.inputs(super::ATTACK, &[0, 2]);
        assert_eq!(
            inputs,
            vec![
                crate::Input {
                    seat: 0,
                    turn: super::DEFEND,
                    name: "crackme.1".to_string(),
                },
                crate::Input {
                    seat: 2,
                    turn: super::DEFEND,
                    name: "crackme.3".to_string(),
                },
            ]
        );
        assert!(super::Crackme.inputs(super::DEFEND, &[1]).is_empty());
        assert_eq!(super::seat_of("crackme.3"), Some(2));
        assert_eq!(super::seat_of("keygen"), None);
    }

    #[test]
    fn a_pairing_is_two_rounds_won_by_cracking_or_forfeit() {
        let first = [played(true, &[]), played(false, &["crackme.2"])];
        let second = [played(true, &[]), played(false, &[])];
        let outcome = super::Crackme.outcome((0, &first), (1, &second)).unwrap();
        assert_eq!(
            outcome.tally,
            ava_wire::Tally {
                won: 2,
                drawn: 0,
                lost: 0
            }
        );
        assert_eq!(outcome.reason, None);

        let second = [played(false, &[]), played(false, &["crackme.1"])];
        let outcome = super::Crackme.outcome((0, &first), (1, &second)).unwrap();
        assert_eq!(
            outcome.tally,
            ava_wire::Tally {
                won: 1,
                drawn: 0,
                lost: 1
            }
        );
        assert_eq!(outcome.reason.as_deref(), Some("seat 2 left no crackme"));

        let none = [played(false, &[]), played(false, &[])];
        let outcome = super::Crackme.outcome((0, &none), (1, &none)).unwrap();
        assert_eq!(outcome.tally.rounds(), 0);
    }

    #[test]
    fn the_sample_is_distinct_numbers() {
        let numbers = super::sample();
        assert_eq!(numbers.len(), super::SAMPLE);
        for (index, number) in numbers.iter().enumerate() {
            assert!(!numbers[..index].contains(number));
        }
    }

    #[test]
    fn an_altered_key_differs_in_its_first_character() {
        assert_eq!(super::altered("abc"), "bbc");
        assert_eq!(super::altered("zzz"), "azz");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_confined_binary_reaches_the_system_and_nothing_else() {
        let secret =
            std::env::temp_dir().join(format!("ava-crackme-secret-{}", std::process::id()));
        std::fs::write(&secret, "secret").unwrap();
        let cat = std::path::Path::new("/bin/cat");

        let outside = super::run_with_timeout(cat, &[secret.to_str().unwrap()]);
        let inside = super::run_with_timeout(cat, &["/etc/passwd"]);
        std::fs::remove_file(&secret).unwrap();

        assert!(!outside.unwrap().unwrap().0.success());
        assert!(inside.unwrap().unwrap().0.success());
    }
}
