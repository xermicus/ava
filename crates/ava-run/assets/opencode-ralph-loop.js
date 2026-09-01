import { existsSync } from "node:fs";

const PROMPT = __AVA_PROMPT__;
const VARIANT = __AVA_VARIANT__;
const LAST_CALL_MARKER = "/home/agent/last-call";
const LAST_CALL = "Time is up. Commit and push what you have right now.";

export const RalphLoop = async ({ client, directory }) => {
    let session = null;
    let iteration = 1;
    let lastCalled = false;

    const prompt = async (text) => {
        const body = { parts: [{ type: "text", text }] };
        if (VARIANT) {
            body.variant = VARIANT;
        }
        for (;;) {
            try {
                await client.session.promptAsync({ path: { id: session }, query: { directory }, body });
                return;
            } catch (error) {
                await new Promise((resolve) => setTimeout(resolve, 5000));
            }
        }
    };

    const loop = async () => {
        if (lastCalled) return;
        if (existsSync(LAST_CALL_MARKER)) {
            lastCalled = true;
            await prompt(LAST_CALL);
            return;
        }
        iteration += 1;
        await prompt("Loop iteration " + iteration + ".\n\n" + PROMPT);
    };

    setTimeout(async () => {
        for (;;) {
            try {
                const created = await client.session.create({ body: {}, query: { directory } });
                session = (created.data ?? created).id;
                break;
            } catch (error) {
                await new Promise((resolve) => setTimeout(resolve, 5000));
            }
        }
        await prompt(PROMPT);
    }, 0);

    return {
        event: async ({ event }) => {
            if (event.type === "permission.asked") {
                await client.postSessionIdPermissionsPermissionId({
                    body: { response: "once" },
                    path: { id: event.properties.sessionID, permissionID: event.properties.id },
                });
                return;
            }
            if (event.type === "question.asked") {
                await fetch(
                    `http://127.0.0.1:4096/api/session/${event.properties.sessionID}/question/${event.properties.id}/reply`,
                    { method: "POST", headers: { "content-type": "application/json" },
                      body: JSON.stringify({ answers: event.properties.questions.map(() => []) }) },
                );
                return;
            }
            if (event.type === "session.error") {
                const errored = event.properties && event.properties.sessionID;
                if (errored && errored !== session) return;
                await loop();
                return;
            }
            if (event.type !== "session.status") return;
            if (event.properties.sessionID !== session) return;
            const status = event.properties.status.type;
            if (status !== "idle" && status !== "error") return;
            await loop();
        },
    };
};
