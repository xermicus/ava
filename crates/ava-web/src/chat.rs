//! The chat bus: what the proxies publish and the browsers read.

/// The characters kept per run for a browser subscribing mid answer.
const KEPT_CHARACTERS: usize = 8 * 1024;

/// The runs kept at once, the quietest dropped for a new one.
const KEPT_RUNS: usize = 8;

/// How long a subscriber waits before a comment is sent to it.
const KEEPALIVE: std::time::Duration = std::time::Duration::from_secs(15);

/// The server sent event carrying one delta, and the comment carrying none.
const EVENT_PREFIX: &str = "data: ";
const KEPT_EVENT_PREFIX: &str = "event: kept\ndata: ";
const EVENT_SUFFIX: &str = "\n\n";
const KEEPALIVE_EVENT: &str = ":\n\n";

/// The answer the events follow, written before the first one.
const PREAMBLE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
     Cache-Control: no-cache\r\nConnection: close\r\n\r\n";

/// One run: what was published and who is reading.
struct Channel {
    run: String,
    kept: String,
    /// When it was last published to.
    published: std::time::Instant,
    subscribers: Vec<std::sync::mpsc::Sender<String>>,
}

static CHANNELS: std::sync::Mutex<Vec<Channel>> = std::sync::Mutex::new(Vec::new());

/// The channel of `run`, opened on the first word of it.
fn channel<'a>(channels: &'a mut Vec<Channel>, run: &str) -> &'a mut Channel {
    if let Some(index) = channels.iter().position(|channel| channel.run == run) {
        return &mut channels[index];
    }

    if channels.len() >= KEPT_RUNS
        && let Some(quietest) = channels
            .iter()
            .enumerate()
            .min_by_key(|(_, channel)| channel.published)
            .map(|(index, _)| index)
    {
        channels.remove(quietest);
    }

    channels.push(Channel {
        run: run.to_string(),
        kept: String::new(),
        published: std::time::Instant::now(),
        subscribers: Vec::new(),
    });

    channels.last_mut().expect("the channel was just opened")
}

/// Hand `text` to everyone reading `run` and keep it for whoever comes next.
pub(crate) fn publish(run: &str, text: &str) {
    let mut channels = CHANNELS.lock().expect("the channels are never poisoned");
    let channel = channel(&mut channels, run);

    channel.published = std::time::Instant::now();
    channel.kept.push_str(text);
    if let Some((index, _)) = channel.kept.char_indices().nth_back(KEPT_CHARACTERS) {
        channel.kept = channel.kept[index..].to_string();
    }

    // A send fails once the reader is gone, which is what drops a subscriber.
    channel
        .subscribers
        .retain(|subscriber| subscriber.send(text.to_string()).is_ok());
}

/// What is kept for `run` and everything published to it from now on.
fn subscribe(run: &str) -> (String, std::sync::mpsc::Receiver<String>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut channels = CHANNELS.lock().expect("the channels are never poisoned");
    let channel = channel(&mut channels, run);

    channel.subscribers.push(sender);

    (channel.kept.clone(), receiver)
}

/// One delta as an event.
fn event(text: &str) -> String {
    format!("{EVENT_PREFIX}{}{EVENT_SUFFIX}", quoted(text))
}

/// What was kept, as the event the browser does not count as text arriving.
fn kept_event(text: &str) -> String {
    format!("{KEPT_EVENT_PREFIX}{}{EVENT_SUFFIX}", quoted(text))
}

/// The text as JSON, since an event separates on newlines.
fn quoted(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
}

/// Answer `request` with the events of `run` on the socket itself, which a
/// buffered answer would only reach the browser at the end of.
pub(crate) fn stream(run: &str, request: tiny_http::Request) -> std::io::Result<()> {
    use std::io::Write;

    let (kept, receiver) = subscribe(run);
    let mut socket = request.into_writer();

    socket.write_all(PREAMBLE.as_bytes())?;
    if !kept.is_empty() {
        socket.write_all(kept_event(&kept).as_bytes())?;
    }
    socket.flush()?;

    loop {
        let written = match receiver.recv_timeout(KEEPALIVE) {
            Ok(text) => event(&text),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => KEEPALIVE_EVENT.to_string(),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        };

        socket.write_all(written.as_bytes())?;
        socket.flush()?;
    }
}
