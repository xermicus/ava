//! The versioned records the core produces and consumes: what a run was
//! started with and left behind, and what a tournament seats and plays.
//!
//! The records hold facts. Points, ratings and standings are derived from
//! them wherever they are shown and never stored, so the knobs deriving them
//! can change without touching a record.

/// The version of the wire format every record carries.
pub const VERSION: u32 = 1;

/// The version a record written before the wire format reads as.
const UNVERSIONED: u32 = 0;

/// The scheme pairing the seats of a tournament: every pair of seats fights once a round.
pub const ROUND_ROBIN: &str = "round-robin";

fn unversioned() -> u32 {
    UNVERSIONED
}

/// An agent: a harness paired with a model, asked for a thinking level.
///
/// This is the identity seats hold and ratings key on. The version of the
/// harness is a fact of every run the agent plays, since it is only knowable
/// once the image exists.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Agent {
    pub harness: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
}

impl Agent {
    /// The agent the way it is referred to: the harness on the model, with the
    /// thinking level when one was asked for.
    pub fn label(&self) -> String {
        match &self.thinking {
            Some(thinking) => format!("{} on {} at {thinking}", self.harness, self.model),
            None => format!("{} on {}", self.harness, self.model),
        }
    }
}

/// What the verifier of a game says about one submission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Verdict {
    /// Whether the submission does what the task asks.
    #[serde(alias = "solved")]
    pub passed: bool,
    /// Why it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Verdict {
    /// The verdict on a submission doing what the task asks.
    pub fn passed() -> Self {
        Self {
            passed: true,
            reason: None,
        }
    }

    /// The verdict on a submission failing the task for `reason`.
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            reason: Some(reason.into()),
        }
    }
}

/// One push the scorer graded, as one line of the attempts log.
///
/// The seconds count from the start of the scoring container, which is the
/// clock every attempt of a run shares.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attempt {
    pub seconds: u64,
    #[serde(flatten)]
    pub verdict: Verdict,
}

/// The rounds of one pairing, from the view of its first seat.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tally {
    pub won: u64,
    pub drawn: u64,
    pub lost: u64,
}

impl Tally {
    /// One round to the first seat: the second forfeited, or a pairing decided
    /// by a single verdict went to the first.
    pub const FIRST_WON: Self = Self {
        won: 1,
        drawn: 0,
        lost: 0,
    };

    /// One round to the second seat.
    pub const SECOND_WON: Self = Self {
        won: 0,
        drawn: 0,
        lost: 1,
    };

    /// The rounds played.
    pub fn rounds(&self) -> u64 {
        self.won + self.drawn + self.lost
    }

    /// The score of the first seat in `[0, 1]`, a win counting one and a draw
    /// half, or nothing when no round was played.
    pub fn score(&self) -> Option<f64> {
        let rounds = self.rounds();
        if rounds == 0 {
            return None;
        }

        Some((self.won as f64 + self.drawn as f64 / 2.0) / rounds as f64)
    }
}

/// The aggregate over every request in the proxy access log of a run.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Metrics {
    pub requests: u64,
    /// The requests answered with a non-200 status.
    pub failed_requests: u64,
    /// The requests a model answered in full without ever reporting its usage,
    /// so the stream was cut short upstream.
    pub truncated_requests: u64,
    /// The requests the client abandoned before the answer was written. The
    /// restart at the end of a turn leaves one of these behind whenever the
    /// agent had a request in flight.
    pub aborted_requests: u64,
    /// The answers an upstream withheld until it had generated all of them.
    pub buffered_requests: u64,
    /// Every distinct host that was requested.
    pub hosts: Vec<String>,
    /// Every distinct model identifier seen in a response body.
    pub served_models: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    /// Content delta events counted as the streams passed, the approximate
    /// volume of the requests whose usage report never arrived.
    pub streamed_deltas: u64,
    /// The account limits of the newest answer that reported them.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub ratelimits: String,
    /// The cost the gateway reported, summed over the answers that carried one.
    pub gateway_cost: f64,
    pub request_bytes: u64,
    pub response_bytes: u64,
    pub request_seconds: f64,
    /// The mean time to the first generated token, over the requests reporting one.
    pub mean_first_token_seconds: f64,
}

/// The entry a run attacks, for a playout an agent plays: the run that kept it
/// and the seconds of the attempt it came from.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Challenge {
    pub run: String,
    pub attempt: u64,
}

/// One run, kept as `runs/<run>/run.json`: written when the run starts and
/// completed when it is over.
///
/// A record without `finished_seconds` is a run still going, or one that
/// broke before it could be completed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Run {
    #[serde(default = "unversioned")]
    pub version: u32,
    pub run: String,
    #[serde(alias = "agent")]
    pub harness: String,
    /// What the harness reports as its version.
    #[serde(default)]
    pub harness_version: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    pub game: String,
    /// The commit the game folder was last changed in.
    #[serde(default)]
    pub game_version: String,
    /// The seconds the run was given, the last call included.
    pub limit_seconds: u64,
    pub started_seconds: u64,
    /// The id of the image the sandbox played on.
    pub image: String,
    /// What the agent was told to start on.
    pub prompt: String,
    pub arguments: Vec<String>,
    /// The names of the variables the sandbox was given, never their values.
    pub variables: Vec<String>,
    /// The entry the run attacked, when it played a pairing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<Challenge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_seconds: Option<u64>,
    /// Every push the scorer graded.
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
}

impl Run {
    /// The agent that played the run.
    pub fn agent(&self) -> Agent {
        Agent {
            harness: self.harness.clone(),
            model: self.model.clone(),
            thinking: self.thinking.clone(),
        }
    }

    /// Whether any push passed the verifier.
    pub fn passed(&self) -> bool {
        self.attempts.iter().any(|attempt| attempt.verdict.passed)
    }

    /// The seconds the run took, once it is over.
    pub fn wall_seconds(&self) -> Option<u64> {
        self.finished_seconds
            .map(|finished| finished.saturating_sub(self.started_seconds))
    }
}

/// A tournament, kept as `tournaments/<name>/tournament.json`: the seats and
/// every round they played.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Tournament {
    #[serde(default = "unversioned")]
    pub version: u32,
    pub name: String,
    pub game: String,
    /// The commit the game folder was last changed in when the tournament was created.
    pub game_version: String,
    /// The scheme pairing the seats, [`ROUND_ROBIN`].
    pub pairing: String,
    /// The seconds every run of the tournament is given.
    pub limit_seconds: u64,
    pub created_seconds: u64,
    /// The lobby: the agent in every seat, by seat number. Two seats may hold
    /// the same agent.
    pub seats: Vec<Agent>,
    pub rounds: Vec<Round>,
}

impl Tournament {
    /// Whether any round was played, which is what fixes the seats.
    pub fn played(&self) -> bool {
        !self.rounds.is_empty()
    }

    /// The rounds that were played to the end.
    pub fn finished_rounds(&self) -> impl Iterator<Item = &Round> {
        self.rounds
            .iter()
            .filter(|round| round.finished_seconds.is_some())
    }
}

/// One round: every seat played a run, and every pairing of the entries fought.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Round {
    pub started_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_seconds: Option<u64>,
    /// One per seat, in seat order.
    pub entries: Vec<Entry>,
    /// In the order they were fought. Empty for a game whose entries stand
    /// alone, where the pairings are derived from the entries when shown.
    pub pairings: Vec<Pairing>,
}

/// The run one seat played in a round, and the attempt whose entry fights for it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub seat: usize,
    pub run: String,
    /// The seconds of the attempt that is the entry of record, or nothing
    /// while the run goes or when no attempt passed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u64>,
}

/// The result of one pairing of two seats.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pairing {
    pub first: usize,
    pub second: usize,
    /// When the pairing was fought, the sequence an order dependent rating follows.
    pub seconds: u64,
    #[serde(flatten)]
    pub tally: Tally,
    /// Why the tally is not the outcome of a fight: a forfeit, or a fight that failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The run that played the pairing, for a playout an agent plays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<String>,
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_legacy_attempt_line_reads_solved_as_passed() {
        let line = r#"{"seconds": 126, "solved": true, "points": 10000}"#;
        let attempt: super::Attempt = serde_json::from_str(line).unwrap();

        assert_eq!(attempt.seconds, 126);
        assert!(attempt.verdict.passed);
        assert_eq!(attempt.verdict.reason, None);
    }

    #[test]
    fn a_legacy_run_record_reads_the_agent_as_the_harness() {
        let record = r#"{
            "run": "pi-1", "agent": "pi", "model": "m", "game": "g", "thinking": null,
            "limit_seconds": 120, "image": "sha256:0", "prompt": "p", "arguments": [],
            "variables": ["X"], "started_seconds": 1
        }"#;
        let run: super::Run = serde_json::from_str(record).unwrap();

        assert_eq!(run.version, super::UNVERSIONED);
        assert_eq!(run.harness, "pi");
        assert!(run.attempts.is_empty());
        assert!(run.metrics.is_none());
    }

    #[test]
    fn a_tally_scores_the_first_seat() {
        let tally = super::Tally {
            won: 2,
            drawn: 1,
            lost: 1,
        };

        assert_eq!(tally.score(), Some(0.625));
        assert_eq!(super::Tally::default().score(), None);
    }
}
