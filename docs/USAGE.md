# relay usage

The terms used here (server, client, address, thread, turn, daemon) are defined in the [README](../README.md).

## Install

```
relay install
```

`install` registers the server to start at every logon of the current user, and starts it immediately, so no logoff is needed. The registration records the path of the `relay` binary that ran `install`, so keep the binary at a stable location, and run `install` again after moving it.

| Platform | Registration | Name | Restart behavior |
|---|---|---|---|
| Windows | Task Scheduler task with a logon trigger, hidden | `Relay` | restarts every minute after a failure, up to 999 times |
| Linux | `systemd --user` unit `~/.config/systemd/user/relay.service` | `relay.service` | `Restart=always` |
| macOS | LaunchAgent `~/Library/LaunchAgents/com.relay.agent.plist` | `com.relay.agent` | restarts after an unsuccessful exit |

The Linux and macOS registrations exist in the code, but a server started on those platforms refuses every send, because the caller check (process inspection) is implemented for Windows only. On any other platform `install` and `uninstall` fail with "no install support for this platform".

If Windows answers access denied, `install` fails and says to run it again from an elevated (administrator) shell. It fails before writing the skills or touching a running server, so a failed `install` changes nothing.

After registering the server, `install` also writes the relay agent skill (a `SKILL.md` that teaches an agent the `relay send` commands) for each agent whose configuration folder exists:

| Agent | Skill file, relative to the folder | Configuration folder |
|---|---|---|
| Claude Code | `skills/relay/SKILL.md` | `~/.claude`, or `CLAUDE_CONFIG_DIR` when set |
| Codex | `skills/relay/SKILL.md` | `~/.codex`, or `CODEX_HOME` when set |

An agent whose configuration folder does not exist is skipped, not created, and `install` prints that it was skipped. Each line `install` prints says what was written or skipped. The skill texts are built into the binary as templates, so `install` does not depend on where the repository is, and running it again replaces a changed skill file. Every command line in a written skill starts with the full path of the `relay` binary that ran `install` (backslashes written as `/`, and the path double-quoted when it contains spaces), so the binary does not need to be on `PATH`. After moving the binary, run `relay install` again from the new location to rewrite the skills. The Claude Code skill covers Codex targets only and points Claude Code sessions to SendMessage for other Claude Code sessions; the Codex skill covers both kinds of target. A running Codex daemon loads skills only when it starts, and `install` does not restart it: after installing the Codex skill, run `codex app-server daemon restart` to make the daemon pick it up. `install` prints this after it writes the Codex skill.

## Start

`install` already starts the server, and while the registration exists the next logon starts it again. There is no start command. To start a registered server again without logging on:

- Windows: `schtasks /Run /TN Relay`.
- Linux: `systemctl --user start relay.service`.
- macOS: `launchctl load ~/Library/LaunchAgents/com.relay.agent.plist` (`install` runs this with `-w`).

To run the server by hand instead:

```
relay bridge
```

Only one server runs at a time: a second `relay bridge` exits with "relay is already running". On Windows that message goes only to the log file, because `relay bridge` detaches from its console before it does anything else; the terminal shows nothing, and the same is true of every later message from the server.

There is no status command. A send fails with "relay is not running" when no server is listening. The Codex daemon is a separate process that must be running for sends from or to Codex; start it with `codex app-server daemon start`. The server connects to the daemon on demand, so it can start before the daemon does.

## Stop

There is no stop command. Choose by what you want:

- Stop and unregister (Windows, Linux and macOS; on any other platform it fails with "no install support for this platform"): `relay uninstall`. On Windows it first removes the task, and from a shell that is not elevated it fails there, with the elevated shell advice, before changing anything: the task, the running server and the skills stay as they were. Once the task is removed it ends the running server (the process on the relay endpoint, including one started by hand) and returns only after that process has exited, so the `relay` binary can be replaced right afterwards; if the process is still running after 10 seconds, `uninstall` fails and names the pid to end with `taskkill /F /PID <pid>`. It then removes the skills. On Linux and macOS it stops the service, then removes the registration. A refusal for elevation leaves the skills in place; any other registration failure still removes them and is reported.
- Stop but keep the registration:
  - Windows: `schtasks /End /TN Relay`. It also exits 0 when the task is not running.
  - Linux: `systemctl --user stop relay.service`.
  - macOS: `launchctl unload ~/Library/LaunchAgents/com.relay.agent.plist` (`uninstall` runs this with `-w`).
- A server started by hand with `relay bridge` on Windows has no console to close. Find it with `Get-Process relay | Select Id,Path` in PowerShell (the installed server has the same image name; tell them apart by `Path`), then end it with `taskkill /F /PID <pid>`. `taskkill /PID <pid>` without `/F` fails with "This process can only be terminated forcefully (with /F option)". On Linux and macOS `relay bridge` does not detach from its terminal.

`uninstall` also removes the two skill files `install` wrote (and their `relay` folder when it is then empty) and prints what it removed; other files in those folders and other skills stay. It leaves the binary and the log files in place.

## Upgrade

Stop the server (see above), replace the `relay` binary at the path the registration records, and start the server again. A registration made from a different path needs `relay install` run from the new binary.

## Send

```
relay send claude <session id> <message>
relay send codex <thread id> <message>
relay send codex <name> <cwd> <message>
```

Run it from inside a Claude Code session or a Codex thread; relay reads its identity from the environment (`CLAUDE_PID` with `CLAUDE_CODE_SESSION_ID`, or `CODEX_THREAD_ID`). A command run from a plain terminal is refused because nobody can be named as the sender.

The message is one argument, so quote it:

```
relay send codex 01a0f7b8-c4f5-7cb2-8fe8-9d67e4d0b2da "please rerun the build"
```

Argument errors by count: with 3 or 4 arguments in a shape that matches no form (for example `relay send claude <id> a b`) the client prints `Error: usage: ...` and exits 1. A thread id followed by two more words is one of those: it is refused with a hint to quote the message, because `<name> <cwd> <message>` would otherwise read the id as a name. With fewer than 3 or more than 4 arguments the argument parser prints its usage text and exits 2. An empty message fails.

Where ids come from: a command run by Claude Code has `CLAUDE_PID` and `CLAUDE_CODE_SESSION_ID` in its environment, and a command run by Codex has `CODEX_THREAD_ID`. The session id is the value of `CLAUDE_CODE_SESSION_ID`, the thread id the value of `CODEX_THREAD_ID`. The sender's address is also printed at the top of every message you receive, ready to answer.

By name:

- `<name>` is matched exactly; `<cwd>` must be an absolute path (`F:\work\app`, not `.`), and a relative one is refused. Case, `\` against `/` and a trailing separator do not matter.
- If no thread matches, relay creates one: started in `<cwd>` with the model, sandbox and approval policy from the user's Codex configuration, given the message as its first turn and then named. A second send to the same name and cwd finds that thread.
- If naming fails or does not happen within 30 seconds, the send is still `delivered` and prints a note with the unnamed thread's id. That thread stays unnamed and relay does not record it, so a second send to the same name and cwd creates another thread; send to the unnamed thread by its id instead.

## Results and exit codes

| Printed | Where | Exit code | Meaning |
|---|---|---|---|
| `delivered <message id>` (plus `note: ...` on a second line when there is one) | stdout | 0 | Handed to the target. |
| `failed: <reason>` | stderr | 1 | Nothing landed. |
| `may not have landed (message id <id>): <reason>` | stderr | 2 | The handoff was sent but its answer never came. relay never resends. |
| `Error: <reason>` | stderr | 1 | The client could not complete the request: no server running, no caller identity in the environment, a wrongly shaped command. |

An argument count the parser rejects (fewer than 3 or more than 4 arguments, for example `relay send claude <id>` with no message) prints usage text and also exits 2, so exit 2 alone does not mean `may not have landed`; check the printed text. A count of 3 or 4 in the wrong shape is an `Error:` and exits 1. The client waits at most 1020 seconds for the server; if the server dies or stays silent it prints `may not have landed` with the message id.

Failure reasons you will meet:

- `ambiguous`: two or more Codex threads share the name and cwd, or two live Claude Code session files share the session id. The reason lists the thread ids or pids; relay never picks one.
- `not reachable`: no live Claude Code session has that id.
- `target busy, retry`: the target's running turn changed during the send and again during the retry.
- `caller refused`: relay could not confirm who is sending. The reason says why (the Claude Code process or the Codex daemon is not an ancestor of the command, a missing or mismatched session file, an unreadable `daemon.pid`, or a Codex thread the daemon does not know). `could not inspect the calling process` and `no caller identity` are refusals as well.
- `refusing to send to yourself`.
- `the cwd must be an absolute path`.

## What the receiver sees

```
<cross-session-message id="<message id>" from="codex <thread id>">
<message>
</cross-session-message>
```

`from` is the sender's address: pass it to `relay send` to answer. When the sender has a display name, a `from-name="<name>"` attribute follows `from`; it is absent when the sender has none. Replies always go to the `from` address, never to the name.

relay reads the name on every send: from the `name` field of the Claude Code session file `~/.claude/sessions/<pid>.json`, or from the Codex `thread/read` answer (`thread.name`, or `thread.agentNickname` when the thread has no name). It removes `"`, `<`, `>` and control, format, line separator and paragraph separator characters, trims the result, and cuts a name longer than 64 characters to its first 64 followed by `…`. A name that is empty after cleaning is omitted.

Into a Codex thread the message starts a turn on an idle thread, is steered into a running turn, or is queued behind a turn that cannot be steered. Into a Claude Code session it is written to the session's inbox pipe, and relay gets no acknowledgment, so `delivered` there means written. relay never waits for the receiver to act and never reports what it does with the message.

## Files and logs

- Log: `relay/relay.log` under the platform data directory (`%APPDATA%\relay\relay.log` on Windows), rotated at 10 MB with 3 rotated files kept.
- Read by relay: `~/.claude/sessions/` (session files `<pid>.json` and peer keys `<pid>.<hash>.key`) and `~/.codex/app-server-daemon/daemon.pid`.
- Written by relay: the log; and by `install` the registration itself: on Windows a temporary task definition `relay-task.xml` in the temp directory (removed afterward), on Linux `~/.config/systemd/user/relay.service`, on macOS `~/Library/LaunchAgents/com.relay.agent.plist`. `uninstall` removes the unit file and the plist on Linux and macOS. `install` also writes `skills/relay/SKILL.md` under the Claude Code and Codex configuration folders (see Install), and `uninstall` removes them.
