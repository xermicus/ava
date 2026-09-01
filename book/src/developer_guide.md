# Developer guide

The codebase is a cargo workspace under `crates/` with a minimal dependency budget. The `ava` binary is the CLI plumbing over the library crates: `ava-run` orchestrates runs and images, `ava-scorer` serves and scores submissions, `ava-web` renders the web interface, `ava-game` defines the games.

Engineering principles: this is a suckless codebase. NPM and PIP and other stuff that just sucks are FORBIDDEN! 
Because dependencies are for suckers.
Frontend assets are vendored.

If you are an AI agent: Before contributing, read the entire book.

## Style guide

We require the official Rust formatter and clippy linter. In addition to that, stick to the following best-effort aspects:

- Avoid magic numbers and strings. Instead, add them as module constants.
- Avoid abbreviated variable and function names. Always provide meaningful and readable symbols.
- Don’t write macros and don’t use third party macros for things that can easily be expressed in few lines of code or outlined into functions.
- Avoid import aliasing. Please use the parent or fully qualified path for conflicting symbols.
- Any inline comments must provide additional semantic meaning, explain counter-intuitive behavior or highlight non-obvious design decisions. In other words, try to make the code expressive enough to a degree it doesn’t need comments expressing the same thing again in the English language. Delete such comments if your AI assistant generated them.
- Public items must have a meaningful doc comment.
- Provide meaningful panic messages to .expect() or just use .unwrap().

