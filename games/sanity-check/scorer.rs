//! The sanity-check game, scored by a submitted palindrome.

const GAME_NAME: &str = "sanity-check";
const SUBMISSION_FILE: &str = "palindrome";
const MAXIMUM_LENGTH: usize = 20;

/// The sanity-check game, scored by a submitted palindrome.
pub struct SanityCheck;

impl crate::Game for SanityCheck {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    /// Score the palindrome submitted as `palindrome`.
    fn score(&self, submission: &std::path::Path) -> std::io::Result<crate::Score> {
        let path = submission.join(SUBMISSION_FILE);

        let contents = match std::fs::read(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(unsolved(&format!("no {SUBMISSION_FILE} in the submission")));
            }
            Err(error) => {
                return Ok(unsolved(&format!(
                    "{SUBMISSION_FILE} cannot be read: {error}"
                )));
            }
        };

        let text = contents.trim_ascii();

        if text.is_empty() {
            return Ok(unsolved(&format!("{SUBMISSION_FILE} is empty")));
        }

        if !text.is_ascii() {
            return Ok(unsolved(&format!("{SUBMISSION_FILE} is not ASCII")));
        }

        if text.len() > MAXIMUM_LENGTH {
            return Ok(unsolved(&format!(
                "{SUBMISSION_FILE} is {} chars, the maximum is {MAXIMUM_LENGTH}",
                text.len()
            )));
        }

        let lowered = text.to_ascii_lowercase();
        let reversed: Vec<u8> = lowered.iter().rev().copied().collect();
        if lowered != reversed {
            return Ok(unsolved(&format!("{SUBMISSION_FILE} is not a palindrome")));
        }

        Ok(crate::Score {
            game: GAME_NAME,
            solved: true,
            points: crate::MAXIMUM_POINTS,
            reason: None,
        })
    }
}

/// The score of a submission which does not solve the task.
fn unsolved(reason: &str) -> crate::Score {
    log::info!("{reason}");

    crate::Score {
        game: GAME_NAME,
        solved: false,
        points: 0,
        reason: Some(reason.to_string()),
    }
}
