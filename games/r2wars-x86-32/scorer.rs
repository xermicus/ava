//! The r2wars games, one per architecture, scored by whether r2wars accepts the warrior.

/// The folder holding the Dockerfile every r2wars game plays on.
const IMAGE: &str = "r2wars";
const ASSEMBLER: &str = "rasm2";
const SUBMISSION_PREFIX: &str = "warrior.";
const SUBMISSION_SUFFIX: &str = ".asm";

/// The most bytes r2wars loads a warrior at.
const WARRIOR_LIMIT: usize = 512;

/// One r2wars game: the warriors of one architecture, as r2wars names it in the file name.
pub struct R2wars {
    name: &'static str,
    architecture: &'static str,
    assembler_arguments: &'static [&'static str],
}

/// The r2wars games.
pub const GAMES: [R2wars; 2] = [
    game("r2wars-x86-32", "x86-32", &["-a", "x86", "-b", "32"]),
    game("r2wars-gb", "gb", &["-a", "gb", "-b", "16"]),
];

const fn game(
    name: &'static str,
    architecture: &'static str,
    assembler_arguments: &'static [&'static str],
) -> R2wars {
    R2wars {
        name,
        architecture,
        assembler_arguments,
    }
}

impl R2wars {
    /// The file the warrior is submitted as, `warrior.<architecture>.asm`.
    fn submission_file(&self) -> String {
        format!(
            "{SUBMISSION_PREFIX}{}{SUBMISSION_SUFFIX}",
            self.architecture
        )
    }

    fn unsolved(&self, reason: String) -> crate::Score {
        log::info!("{reason}");

        crate::Score {
            game: self.name,
            solved: false,
            points: 0,
            reason: Some(reason),
        }
    }
}

impl crate::Game for R2wars {
    fn name(&self) -> &'static str {
        self.name
    }

    fn image(&self) -> Option<&'static str> {
        Some(IMAGE)
    }

    /// Score the submitted assembly the way r2wars loads it.
    fn score(&self, submission: &std::path::Path) -> std::io::Result<crate::Score> {
        let file = self.submission_file();
        let source = submission.join(&file);
        if !source.is_file() {
            return Ok(self.unsolved(format!("no {file} in the submission")));
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
            return Ok(self.unsolved(format!(
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
            return Ok(self.unsolved(format!(
                "{file} assembles to {bytes} bytes, r2wars loads at most {WARRIOR_LIMIT}"
            )));
        }

        log::info!("{file} assembles to {bytes} bytes");

        Ok(crate::Score {
            game: self.name,
            solved: true,
            points: crate::MAXIMUM_POINTS,
            reason: None,
        })
    }
}
