import { existsSync } from "node:fs";

const PROMPT = __AVA_PROMPT__;
const LAST_CALL_MARKER = "/home/agent/last-call";
const LAST_CALL = "Time is up. Commit and push what you have right now.";
const ERROR_DELAY_MILLISECONDS = 5000;
let iteration = 1;
let lastCalled = false;

export default function (pi) {
    pi.on("agent_end", async (event) => {
        if (event.willRetry || lastCalled) {
            return;
        }
        if (existsSync(LAST_CALL_MARKER)) {
            lastCalled = true;
            pi.sendUserMessage(LAST_CALL, { deliverAs: "followUp" });
            return;
        }
        const last = event.messages[event.messages.length - 1];
        if (last && last.errorMessage) {
            await new Promise((resolve) => setTimeout(resolve, ERROR_DELAY_MILLISECONDS));
        }
        iteration += 1;
        pi.sendUserMessage("Loop iteration " + iteration + ".\n\n" + PROMPT, { deliverAs: "followUp" });
    });
}
