#!/bin/bash

# Assemble the git remote of one run: the bare repository with its verifying
# hooks, the clock, the folder keeping the entries of the passing attempts, and
# the http shim serving it. The attempts file is kept as the container output,
# forming runs/<run>/score.log. The arguments are the game, the turn of it the
# run plays and the seconds a verification may take.

set -euo pipefail

GAME="$1"
TURN="$2"
TIMEOUT="$3"
HOME_DIR=/home/agent
REPOSITORY="$HOME_DIR/task.git"
SCAFFOLD=/opt/scaffold

# The shim drops to the sandbox user, whose processes must not look in /root.
export HOME="$HOME_DIR"

echo "$GAME" > "$HOME_DIR/game"
echo "$TURN" > "$HOME_DIR/turn"
echo "$TIMEOUT" > "$HOME_DIR/timeout"
date +%s > "$HOME_DIR/started"
touch "$HOME_DIR/attempts.jsonl"
mkdir -p "$HOME_DIR/entries"

git init --quiet --bare "$REPOSITORY"
git -C "$REPOSITORY" config http.receivepack true

# Seed master from the mounted task before the hooks exist, so the protection
# below never needs an exception. The scaffold a game lays into its image goes
# in with the task. The inputs of the turn, the entries of the other seats, go
# into the workspace under their names.
SEED=$(mktemp -d)
cp -r /home/agent/task/. "$SEED"
if [ -d "$SCAFFOLD" ]; then
	cp -r "$SCAFFOLD/." "$SEED"
fi
cp /home/agent/README.md "$SEED/README.md"
if [ -d "$HOME_DIR/inputs" ]; then
	for entry in "$HOME_DIR/inputs"/*; do
		cp "$entry" "$SEED/"
		chmod 755 "$SEED/$(basename "$entry")"
	done
fi

{
	echo "AGENTS.md"
	echo "CLAUDE.md"
	echo ".claude/"
} >> "$SEED/.gitignore"

git -C "$SEED" init --quiet --initial-branch master
# --force so a scaffold that ships its own .gitignore, as tinycc does, still
# seeds its whole tree: the ignore rules stay for the agent's own commits.
git -C "$SEED" add --all --force
git -C "$SEED" -c user.name=ci -c user.email=ci@ava commit --quiet --message task
git -C "$SEED" push --quiet "$REPOSITORY" master
rm -rf "$SEED"

cp "$HOME_DIR/hooks/update" "$HOME_DIR/hooks/post-receive" "$REPOSITORY/hooks/"
chmod 755 "$REPOSITORY/hooks/update" "$REPOSITORY/hooks/post-receive"
chown -R 1000:1000 "$REPOSITORY" "$HOME_DIR/game" "$HOME_DIR/turn" "$HOME_DIR/timeout" "$HOME_DIR/started" "$HOME_DIR/attempts.jsonl" "$HOME_DIR/entries"

ava remote &

# As the sandbox user, the one a push is scored under, so the data prepare
# leaves in the temporary directory belongs to whoever a verification reads it.
runuser -u agent -- ava score --game "$GAME" --prepare &

exec tail --lines +1 --follow "$HOME_DIR/attempts.jsonl"
