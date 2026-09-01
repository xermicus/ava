#!/bin/bash

# Assemble the git remote of one run: the bare repository with its scoring
# hooks, the clock, and the http shim serving it. The attempts file is kept
# as the container output, forming runs/<run>/score.log.

set -euo pipefail

GAME="$1"
HOME_DIR=/home/agent
REPOSITORY="$HOME_DIR/task.git"

# The shim drops to the sandbox user, whose processes must not look in /root.
export HOME="$HOME_DIR"

echo "$GAME" > "$HOME_DIR/game"
date +%s > "$HOME_DIR/started"
touch "$HOME_DIR/attempts.jsonl"

git init --quiet --bare "$REPOSITORY"
git -C "$REPOSITORY" config http.receivepack true

# Seed master from the mounted task before the hooks exist, so the protection
# below never needs an exception.
SEED=$(mktemp -d)
cp -r /home/agent/task/. "$SEED"
cp /home/agent/README.md "$SEED/README.md"
git -C "$SEED" init --quiet --initial-branch master
git -C "$SEED" add --all
git -C "$SEED" -c user.name=ci -c user.email=ci@ava commit --quiet --message task
git -C "$SEED" push --quiet "$REPOSITORY" master
rm -rf "$SEED"

cp "$HOME_DIR/hooks/update" "$HOME_DIR/hooks/post-receive" "$REPOSITORY/hooks/"
chmod 755 "$REPOSITORY/hooks/update" "$REPOSITORY/hooks/post-receive"
chown -R 1000:1000 "$REPOSITORY" "$HOME_DIR/game" "$HOME_DIR/started" "$HOME_DIR/attempts.jsonl"

ava remote &

exec tail --lines +1 --follow "$HOME_DIR/attempts.jsonl"
