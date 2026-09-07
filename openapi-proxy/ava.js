/*
 * WARNING: THIS FILE IS STINKING AI SLOP FROM ONE OF THE WORST LLM (claude opus 5).
 *
 * Harvest per-request metrics out of the bodies as they stream past, and
 * publish the generated text to the bus as it goes.
 *
 * The filters never parse a body as JSON. A streamed answer is a sequence of
 * server sent events, so no single chunk is a complete document, and every
 * field we want is a small set of keys we can scan for. Scanning works the
 * same on a streamed answer and on a single document one.
 */

/* Long enough to hold any key we scan for together with its value, so a field
 * split across two chunks is still matched once the chunks are joined. */
const RETAINED_TAIL_BYTES = 256;

/* An agent that writes JSON into its own prompt reaches both bodies verbatim,
 * so a model identifier can be injected. Recording the distinct values instead
 * of the last one leaves the injected identifier next to the real one, where
 * the verifier can see it. The limit keeps such a body from growing the log
 * line without bound. */
const DISTINCT_MODEL_LIMIT = 8;

/* A model name can hold spaces, as `DeepSeek V4 Flash (Wyna)` does. */
const MODEL_SEPARATOR = '\t';
const UNSET_ELAPSED = '0';
const MODEL_KEY = 'model';

/* Each entry names a log variable and the usage keys feeding it, in the
 * Anthropic shape first and the OpenAI shape second. */
const INPUT_KEYS = ['input_tokens', 'prompt_tokens'];
const TOKEN_FIELDS = [
    ['ava_input_tokens', INPUT_KEYS],
    ['ava_output_tokens', ['output_tokens', 'completion_tokens']],
    ['ava_cache_read_tokens', ['cache_read_input_tokens', 'cached_tokens']],
    ['ava_cache_write_tokens', ['cache_creation_input_tokens']],
];

/* The OpenAI shapes count the cached tokens inside the input tokens, the
 * Anthropic shape reports them apart, so the input tokens are the uncached
 * ones on every shape once these are taken out. */
const OPENAI_CACHED_KEY = 'cached_tokens';

/* An event carrying generated content, one pattern per streaming shape: the
 * Anthropic shape, the OpenAI chat shapes and the OpenAI responses shape,
 * reasoning deltas included. The first one is the time to the first token, and
 * counting them approximates the volume of a stream that is cut before its
 * usage report.
 *
 * One event can match more than one pattern. An Anthropic event whose delta
 * carries the text key directly matches the first two, so the counts are taken
 * one pattern at a time and the largest of them is the number of events. */
const DELTA_PATTERNS = [
    '"type"\\s*:\\s*"content_block_delta"',
    '"delta"\\s*:\\s*\\{\\s*"(?:content|reasoning_content|text)"',
    '"type"\\s*:\\s*"response\\.(?:output_text|reasoning_text|reasoning_summary_text)\\.delta"',
];

const FIRST_TOKEN_MARKER = new RegExp(DELTA_PATTERNS.join('|'));

/* The generated text of a delta event, one pattern per streaming shape, in the
 * order of DELTA_PATTERNS. The captured value is escaped the way the source
 * escaped it, so it is written on as it stands and read back as JSON. */
const TEXT_PATTERNS = [
    '"(?:text|thinking)_delta"[^}]*?"(?:text|thinking)"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)"',
    '"delta"\\s*:\\s*\\{[^}]*?"(?:content|reasoning_content|text)"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)"',
    '"type"\\s*:\\s*"response\\.(?:output_text|reasoning_text|reasoning_summary_text)\\.delta"[^}]*?"delta"\\s*:\\s*"((?:[^"\\\\]|\\\\.)*)"',
];

/* Where the generated text is published, named in the environment when a web
 * interface is up. Nothing is published without it. */
const BUS = process.env.AVA_BUS;

/* The queue the filter fills and the timer drains. A body filter may not do
 * anything asynchronous, and every request and every tick runs in a VM of its
 * own, so the queue is the shared zone and not a variable of this module.
 *
 * Past the limit nobody is reading, and the newest text is the one worth
 * keeping, since it is what a subscriber watches arrive. */
const QUEUE_ZONE = 'chat';
const QUEUE_KEY = 'queued';
const QUEUE_LIMIT_CHARACTERS = 64 * 1024;

function matchAll(text, pattern) {
    const values = [];
    let match;

    while ((match = pattern.exec(text)) !== null) {
        values.push(match[1]);
    }

    return values;
}

/*
 * The leading quote in the pattern is what keeps a key from matching inside a
 * longer one, such as `input_tokens` inside `cache_read_input_tokens`.
 */
function modelNames(text) {
    return matchAll(text, new RegExp('"' + MODEL_KEY + '"\\s*:\\s*"([^"]*)"', 'g'));
}

/*
 * The last integer held by `key` inside a usage object.
 *
 * An Anthropic stream reports usage twice, in the opening event and in the
 * closing one, and the closing report is the final count. Requiring the
 * enclosing `"usage": {` keeps generated text that happens to contain the key
 * out of the count. The flat pattern is the fallback for a usage object that
 * nests another object ahead of the key.
 */
function usageInteger(text, key) {
    const scoped = matchAll(
        text,
        new RegExp('"usage"\\s*:\\s*\\{[^{}]*"' + key + '"\\s*:\\s*(\\d+)', 'g'),
    );

    if (scoped.length > 0) {
        return scoped[scoped.length - 1];
    }

    const flat = matchAll(text, new RegExp('"' + key + '"\\s*:\\s*(\\d+)', 'g'));

    return flat.length > 0 ? flat[flat.length - 1] : null;
}

/*
 * Append `value` to `name` unless it is already recorded.
 */
function recordDistinct(request, name, value) {
    const recorded = request.variables[name];

    if (recorded === '') {
        request.variables[name] = value;
        return;
    }

    const seen = recorded.split(MODEL_SEPARATOR);
    if (seen.length >= DISTINCT_MODEL_LIMIT) {
        return;
    }

    for (let index = 0; index < seen.length; index++) {
        if (seen[index] === value) {
            return;
        }
    }

    request.variables[name] = recorded + MODEL_SEPARATOR + value;
}

/*
 * Record `elapsed` only for the first chunk it is called on.
 *
 * An elapsed time is always formatted with decimals, so it never equals the
 * unset marker and the first recorded value is the one that survives.
 */
function recordOnce(request, name, elapsed) {
    if (request.variables[name] === UNSET_ELAPSED) {
        request.variables[name] = elapsed;
    }
}

/*
 * The input tokens of a usage object in `window` less the cached ones, or
 * null unless the window holds both, so a usage split across two chunks is
 * reduced once.
 */
function uncachedInput(window) {
    const cached = usageInteger(window, OPENAI_CACHED_KEY);
    if (cached === null) {
        return null;
    }

    for (let key = 0; key < INPUT_KEYS.length; key++) {
        const input = usageInteger(window, INPUT_KEYS[key]);

        if (input !== null) {
            return String(Math.max(0, Number(input) - Number(cached)));
        }
    }

    return null;
}

function recordModels(request, name, window) {
    const models = modelNames(window);

    for (let index = 0; index < models.length; index++) {
        recordDistinct(request, name, models[index]);
    }
}

/*
 * The delta events in `window` that end past the first `tail` bytes.
 *
 * A match ending inside the retained tail was counted on the chunk that
 * carried it, while one reaching past the tail ends in new bytes, so every
 * event is counted exactly once, split across two chunks or not.
 */
function countDeltas(window, tail) {
    let count = 0;

    for (let shape = 0; shape < DELTA_PATTERNS.length; shape++) {
        const pattern = new RegExp(DELTA_PATTERNS[shape], 'g');
        let counted = 0;
        let match;

        while ((match = pattern.exec(window)) !== null) {
            if (match.index + match[0].length > tail) {
                counted++;
            }
        }

        if (counted > count) {
            count = counted;
        }
    }

    return count;
}

/*
 * The generated text of the delta events in `window` that end past the first
 * `tail` bytes, joined, escaped as the source escaped it.
 *
 * One event matches at most one shape, so the shape with the most matches is
 * the one the stream speaks and its matches are the text in order.
 */
function deltaText(window, tail) {
    let text = '';

    for (let shape = 0; shape < TEXT_PATTERNS.length; shape++) {
        const pattern = new RegExp(TEXT_PATTERNS[shape], 'g');
        let spoken = '';
        let match;

        while ((match = pattern.exec(window)) !== null) {
            if (match.index + match[0].length > tail) {
                spoken += match[1];
            }
        }

        if (spoken.length > text.length) {
            text = spoken;
        }
    }

    return text;
}

/*
 * Hold `text` until the next tick of the timer.
 */
function queue(text) {
    if (text === '' || !BUS) {
        return;
    }

    const queue = ngx.shared[QUEUE_ZONE];
    const queued = (queue.get(QUEUE_KEY) || '') + text;

    queue.set(QUEUE_KEY, queued.slice(-QUEUE_LIMIT_CHARACTERS));
}

/*
 * Publish what the filters scraped since the last tick.
 *
 * The text is escaped as the source escaped it, so what the queue holds is a
 * JSON string body already and the bus reads it back as one. A publish that
 * fails is an interface that went away, and the text goes with it.
 */
async function publish() {
    if (!BUS) {
        return;
    }

    const queued = ngx.shared[QUEUE_ZONE].pop(QUEUE_KEY);
    if (!queued) {
        return;
    }

    try {
        await ngx.fetch(BUS, { method: 'POST', body: '{"chat":"' + queued + '"}' });
    } catch (error) {
        ngx.log(ngx.WARN, 'the chat was not published: ' + error);
    }
}

/* The backends report the account limits in their answer headers, and the
 * gateway the budget of the key. The last captured set is the state of the
 * account as of the newest request. */
const LIMIT_HEADER_PREFIXES = ['anthropic-ratelimit-', 'x-ratelimit-', 'x-litellm-key-'];

/* LiteLLM forwards the headers of the provider under this prefix. */
const FORWARDED_PREFIX = 'llm_provider-';

const COST_HEADERS = ['x-litellm-response-cost-original', 'x-litellm-response-cost'];

const NAME_SEPARATOR = ' ';

/* An answer reporting neither limits nor a cost logs its header names instead. */
function captureLimits(request) {
    const limits = [];
    const names = [];
    const headers = {};

    for (const name in request.headersOut) {
        const lowered = name.toLowerCase();
        headers[lowered] = request.headersOut[name];
        names.push(lowered);

        const unwrapped = lowered.startsWith(FORWARDED_PREFIX)
            ? lowered.slice(FORWARDED_PREFIX.length)
            : lowered;
        for (let index = 0; index < LIMIT_HEADER_PREFIXES.length; index++) {
            if (unwrapped.startsWith(LIMIT_HEADER_PREFIXES[index])) {
                limits.push(unwrapped + '=' + request.headersOut[name]);
            }
        }
    }

    for (let index = 0; index < COST_HEADERS.length; index++) {
        if (headers[COST_HEADERS[index]] !== undefined) {
            request.variables.ava_gateway_cost = headers[COST_HEADERS[index]];
            break;
        }
    }

    if (limits.length > 0) {
        request.variables.ava_ratelimits = limits.sort().join(NAME_SEPARATOR);
    } else if (request.variables.ava_gateway_cost === '') {
        request.variables.ava_gateway_headers = names.sort().join(NAME_SEPARATOR);
    }
}

function captureResponse(request, data, flags) {
    const window = request.variables.ava_response_tail + data;
    const elapsed = request.variables.request_time;

    if (data.length > 0) {
        recordOnce(request, 'ava_first_byte_seconds', elapsed);

        if (FIRST_TOKEN_MARKER.test(window)) {
            recordOnce(request, 'ava_first_token_seconds', elapsed);
        }
    }

    recordModels(request, 'ava_served_models', window);

    const tail = request.variables.ava_response_tail.length;
    const deltas = countDeltas(window, tail);
    if (deltas > 0) {
        request.variables.ava_streamed_deltas =
            String(Number(request.variables.ava_streamed_deltas) + deltas);
    }

    queue(deltaText(window, tail));

    for (let field = 0; field < TOKEN_FIELDS.length; field++) {
        const name = TOKEN_FIELDS[field][0];
        const keys = TOKEN_FIELDS[field][1];

        for (let key = 0; key < keys.length; key++) {
            const tokens = usageInteger(window, keys[key]);

            if (tokens !== null) {
                request.variables[name] = tokens;
            }
        }
    }

    const uncached = uncachedInput(window);
    if (uncached !== null) {
        request.variables.ava_input_tokens = uncached;
    }

    request.variables.ava_response_tail = window.slice(-RETAINED_TAIL_BYTES);
    request.sendBuffer(data, flags);
}

export default { captureResponse, captureLimits, publish };
