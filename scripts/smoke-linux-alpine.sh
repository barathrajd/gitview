#!/bin/sh
# Runs inside alpine:latest: opens a repository with Gitview and an agent pane
# in a pseudo-terminal. The agent checks its socket is private and asks
# Gitview for the status through `gitview mcp`. Called by
# smoke-linux-release.sh.
set -eu
apk add -q git util-linux-misc >/dev/null

mkdir /repository && cd /repository
git init -q -b main
git -c user.name=smoke -c user.email=smoke@example.com commit -q --allow-empty -m first
echo change > new.txt

cat >/agent.sh <<'AGENT'
ls -l "$GITVIEW_SOCKET" | cut -c1-10 >/tmp/socket-mode
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{}}}' |
  gitview mcp >/tmp/status
sleep 3
AGENT

# `q` arrives after the agent has run; Gitview must then quit cleanly.
(sleep 4; printf q) |
  TERM=xterm script -qec "stty rows 40 cols 160 </dev/tty; gitview --run sh /agent.sh" /dev/null >/dev/null 2>&1 ||
  { echo "Gitview did not exit cleanly"; exit 1; }

[ "$(cat /tmp/socket-mode)" = "srw-------" ] || { echo "socket mode: $(cat /tmp/socket-mode)"; exit 1; }
grep -q '\\"path\\":\\"new.txt\\"' /tmp/status || { echo "get_status: $(cat /tmp/status)"; exit 1; }
grep -q '"isError":false' /tmp/status || { echo "get_status: $(cat /tmp/status)"; exit 1; }
echo "alpine run passed"
