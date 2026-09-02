# Loop task

## Instructions

- Details for this task are in `task.md`. Implement what `task.md` asks for.
- Work right here in your workdir (`/home/agent/workspace`).
- Your workspace is a clone of `http://git:8080/task.git`. Work on the `task` branch, `master` is protected.
- Often commit and push the `task` branch to run the CI pipeline. The push output reports the CI run.
- Keep improving the solution and commit + push subsequent submissions after the first passing one.

## Hints

- The best submission counts, not necessarily the last one.
- If you think you've completed the task with an optimal solution or the solution is saturated and wasting more tokens is no longer viable: `git tag release && git push origin release` to notify the supervisor.
- Use whatever you want and use or not use any tools, programs installed, sub agents ad-hoc scripts, whatever.
- You do not have internet connection. Don't waste time trying to do stuff that requires an upstream link. Don't waste tokens to debug or find upstream connection.
- Set sensible timeouts for tool calls especially when testing your task outcome. This prevents you from entering devastating infinite loops.
- Writeable dirs are capped aggressively in size, prevent filling them up.
