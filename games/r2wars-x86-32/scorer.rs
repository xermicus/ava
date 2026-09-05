//! The r2wars games, one per architecture: a warrior is verified by whether
//! r2wars accepts it and fights the other warriors in the playout.

/// The folder holding the Dockerfile every r2wars game plays on.
const IMAGE: &str = "r2wars";
const ASSEMBLER: &str = "rasm2";
const FIGHTER: &str = "r2wars";
const FIGHT_OPTION: &str = "--fight";

/// The most bytes r2wars loads a warrior at.
const WARRIOR_LIMIT: usize = 512;

/// r2wars names a warrior by its file and reads the architecture out of the
/// name, so the two entries of a fight, which share one file name, are staged
/// under these two.
const FIRST_WARRIOR: &str = "first";
const SECOND_WARRIOR: &str = "second";
const SOURCE_SUFFIX: &str = ".asm";
const STAGE_PREFIX: &str = "ava-fight-";

/// How the headless fight reports a round.
const ROUND_PREFIX: &str = "round ";
const WIN_MARKER: &str = " wins after ";
const TIMEOUT_MARKER: &str = ": timeout after ";

/// One r2wars game: the warriors of one architecture, as r2wars names it in the file name.
pub struct R2wars {
    name: &'static str,
    architecture: &'static str,
    /// The file the warrior is submitted as, `warrior.<architecture>.asm`.
    entry: &'static str,
    assembler_arguments: &'static [&'static str],
}

/// The r2wars games.
pub const GAMES: [R2wars; 2] = [
    R2wars {
        name: "r2wars-x86-32",
        architecture: "x86-32",
        entry: "warrior.x86-32.asm",
        assembler_arguments: &["-a", "x86", "-b", "32"],
    },
    R2wars {
        name: "r2wars-gb",
        architecture: "gb",
        entry: "warrior.gb.asm",
        assembler_arguments: &["-a", "gb", "-b", "16"],
    },
];

impl R2wars {
    /// The name r2wars gives the warrior staged as `stem`.
    fn warrior_name(&self, stem: &str) -> String {
        format!("{stem}.{}", self.architecture)
    }
}

impl crate::Game for R2wars {
    fn name(&self) -> &'static str {
        self.name
    }

    fn image(&self) -> Option<&'static str> {
        Some(IMAGE)
    }

    fn entry(&self) -> &'static str {
        self.entry
    }

    fn playout(&self) -> crate::Playout {
        crate::Playout::Automated
    }

    /// Verify the submitted assembly the way r2wars loads it.
    fn verify(
        &self,
        submission: &std::path::Path,
        _challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let file = self.entry;
        let source = submission.join(file);
        if !source.is_file() {
            return Ok(crate::failed(format!("no {file} in the submission")));
        }

        let output = std::process::Command::new(ASSEMBLER)
            .args(self.assembler_arguments)
            .arg("-f")
            .arg(&source)
            .output()?;
        let hex: String = String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .collect();
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !output.status.success()
            || hex.is_empty()
            || !hex.len().is_multiple_of(2)
            || !hex.chars().all(|character| character.is_ascii_hexdigit())
        {
            return Ok(crate::failed(format!(
                "{ASSEMBLER} rejects {file}: {}",
                if error.is_empty() {
                    "nothing assembled"
                } else {
                    &error
                }
            )));
        }

        let bytes = hex.len() / 2;
        if bytes > WARRIOR_LIMIT {
            return Ok(crate::failed(format!(
                "{file} assembles to {bytes} bytes, r2wars loads at most {WARRIOR_LIMIT}"
            )));
        }

        log::info!("{file} assembles to {bytes} bytes");

        Ok(ava_wire::Verdict::passed())
    }

    /// Play `combats` combats between the two warriors, each best of three
    /// rounds on random load positions, and tally every round.
    fn fight(
        &self,
        first: &std::path::Path,
        second: &std::path::Path,
        combats: u64,
    ) -> std::io::Result<ava_wire::Tally> {
        let stage = std::env::temp_dir().join(format!("{STAGE_PREFIX}{}", std::process::id()));
        std::fs::create_dir_all(&stage)?;

        let staged_first = stage.join(format!(
            "{}{SOURCE_SUFFIX}",
            self.warrior_name(FIRST_WARRIOR)
        ));
        let staged_second = stage.join(format!(
            "{}{SOURCE_SUFFIX}",
            self.warrior_name(SECOND_WARRIOR)
        ));
        std::fs::copy(first, &staged_first)?;
        std::fs::copy(second, &staged_second)?;

        let mut tally = ava_wire::Tally::default();
        for _ in 0..combats {
            let output = std::process::Command::new(FIGHTER)
                .arg(FIGHT_OPTION)
                .arg(&staged_first)
                .arg(&staged_second)
                .output();
            let output = match output {
                Ok(output) => output,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(&stage);
                    return Err(error);
                }
            };

            let _ = std::io::Write::write_all(&mut std::io::stderr(), &output.stdout);
            let _ = std::io::Write::write_all(&mut std::io::stderr(), &output.stderr);

            if !output.status.success() {
                let _ = std::fs::remove_dir_all(&stage);
                return Err(std::io::Error::other(format!(
                    "{FIGHTER} failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )));
            }

            let combat = self.tally(&String::from_utf8_lossy(&output.stdout));
            tally.won += combat.won;
            tally.drawn += combat.drawn;
            tally.lost += combat.lost;
        }
        let _ = std::fs::remove_dir_all(&stage);

        Ok(tally)
    }
}

impl R2wars {
    /// The rounds of one combat as the headless fight printed them, from the
    /// view of the first warrior.
    fn tally(&self, printed: &str) -> ava_wire::Tally {
        let mut tally = ava_wire::Tally::default();
        let first = self.warrior_name(FIRST_WARRIOR);
        let second = self.warrior_name(SECOND_WARRIOR);

        for line in printed
            .lines()
            .filter(|line| line.starts_with(ROUND_PREFIX))
        {
            if line.contains(TIMEOUT_MARKER) {
                tally.drawn += 1;
                continue;
            }

            let Some(winner) = line
                .split_once(": ")
                .and_then(|(_, rest)| rest.split_once(WIN_MARKER))
                .map(|(winner, _)| winner)
            else {
                continue;
            };

            if winner == first {
                tally.won += 1;
            } else if winner == second {
                tally.lost += 1;
            }
        }

        tally
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_rounds_are_tallied_from_the_first_view() {
        let printed = "round 1: second.x86-32 wins after 120 cycles, first.x86-32 died: Invalid instruction. int3\n\
                       round 2: first.x86-32 wins after 88 cycles, second.x86-32 died: ESIL trap. jmp eax\n\
                       round 3: timeout after 2001 cycles\n\
                       draw\n";

        let tally = super::GAMES[0].tally(printed);

        assert_eq!(
            tally,
            ava_wire::Tally {
                won: 1,
                drawn: 1,
                lost: 1
            }
        );
    }
}
