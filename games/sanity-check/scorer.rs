//! The sanity-check game, verified by a submitted palindrome.

const GAME_NAME: &str = "sanity-check";
const SUBMISSION_FILE: &str = "palindrome";
const MAXIMUM_LENGTH: usize = 20;

/// The sanity-check game, verified by a submitted palindrome.
pub struct SanityCheck;

impl crate::Game for SanityCheck {
    fn name(&self) -> &'static str {
        GAME_NAME
    }

    fn entry(&self) -> &'static str {
        SUBMISSION_FILE
    }

    /// Verify the palindrome submitted as `palindrome`.
    fn verify(
        &self,
        submission: &std::path::Path,
        _challenge: Option<&std::path::Path>,
    ) -> std::io::Result<ava_wire::Verdict> {
        let path = submission.join(SUBMISSION_FILE);

        let contents = match std::fs::read(&path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(crate::failed(format!(
                    "no {SUBMISSION_FILE} in the submission"
                )));
            }
            Err(error) => {
                return Ok(crate::failed(format!(
                    "{SUBMISSION_FILE} cannot be read: {error}"
                )));
            }
        };

        let text = contents.trim_ascii();

        if text.is_empty() {
            return Ok(crate::failed(format!("{SUBMISSION_FILE} is empty")));
        }

        if !text.is_ascii() {
            return Ok(crate::failed(format!("{SUBMISSION_FILE} is not ASCII")));
        }

        if text.len() > MAXIMUM_LENGTH {
            return Ok(crate::failed(format!(
                "{SUBMISSION_FILE} is {} chars, the maximum is {MAXIMUM_LENGTH}",
                text.len()
            )));
        }

        let lowered = text.to_ascii_lowercase();
        let reversed: Vec<u8> = lowered.iter().rev().copied().collect();
        if lowered != reversed {
            return Ok(crate::failed(format!(
                "{SUBMISSION_FILE} is not a palindrome"
            )));
        }

        Ok(ava_wire::Verdict::passed())
    }
}
