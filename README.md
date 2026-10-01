# relay

relay lets a Claude Code session or a Codex thread send a text message to any other Claude Code session or Codex thread of the same user on this machine. There is one operation, `send`. relay delivers the message, reports whether it was delivered, and does nothing afterward: it stores no messages, follows no replies, and never resends. The receiver answers by sending a message back to the address printed at the top of every message it receives.

For installing, starting, stopping and the command reference, see [docs/USAGE.md](docs/USAGE.md).

## Terms

- **Claude Code session**: one running Claude Code, identified by a session id.
- **Codex thread**: one conversation in Codex, identified by a thread id and optionally a name.
- **Codex app-server daemon** (the daemon): the local process that hosts Codex threads. relay talks to it; it never starts it. The Codex CLI starts it with `codex app-server daemon start`.
- **Turn**: one round of work a Codex thread does in response to a user message.
- **Address**: how a message names its target or its sender, in the same words `relay send` takes: `claude <session id>`, `codex <thread id>`, or `codex <name> <cwd>`.
- **Server**: the long-running `relay bridge` process. **Client**: the short-lived `relay send` command. Both are the same `relay` binary.

## Requirements

- Windows. Sending works on Windows only: process inspection is implemented for Windows only, so on any other platform the server refuses every send with `could not inspect the calling process`. `relay install` has registration code for Linux (systemd) and macOS (launchd); any other platform has no `install` support.
- The Codex CLI on `PATH` (relay runs `codex app-server daemon version` to find the daemon's socket) and the daemon running. Without the daemon, a send from or to Codex fails with the reason; a send from one Claude Code session to another needs no daemon.
- A Rust toolchain to build.

## Build

```
cargo build --release
```

The binary is `target/release/relay` (`relay.exe` on Windows).

## How a send works

`relay send` connects to the server over a local endpoint named `relay-<user name>.sock` (a named pipe on Windows; the user name comes from the `USERNAME` or `USER` environment variable), sends one request, prints the answer, and exits. It never starts the server; when none is running it fails with "relay is not running". Only one server can run at a time: a second `relay bridge` exits with an error.

The server keeps no persisted state. It holds one connection to the daemon (a WebSocket over the daemon's Unix domain socket), one lock per target, and an in-memory record of the threads it created by name, which bridges the short lag before the daemon's thread list shows a new thread.

### Who is calling

The client reports every identity its environment carries: `CLAUDE_PID` together with `CLAUDE_CODE_SESSION_ID` for a Claude Code session, and `CODEX_THREAD_ID` for a Codex thread. A command run outside both has no identity and is refused.

The server never believes the request about who is calling. It reads the client's process id from the connection itself, then walks up that process's parents. A reported identity is kept only if it is backed by an ancestor of the client:

- Claude Code: the `CLAUDE_PID` process must be an ancestor, and `~/.claude/sessions/<pid>.json` must name the same session id and the process start time it recorded.
- Codex: the daemon process named in `~/.codex/app-server-daemon/daemon.pid` (which records the daemon's pid and start time) must be an ancestor and still be running with that start time.

Each step up the chain counts only if the parent started before its child, so a reused process id is never followed. When several identities are kept, the nearest ancestor is the chosen caller. If the chosen caller is a Codex thread, the server then asks the daemon to read that thread (`thread/read`), and a thread the daemon does not know refuses the send. When no identity is kept, the send fails with `caller refused`, and the log records the candidates and the process chain.

### Trust limits

- A Claude Code sender's identity is bound to its session file, so a command cannot claim another session's id.
- A Codex sender's thread id is claimed, not proven. relay checks that the command runs under the daemon and that the thread exists. Any command running under the daemon can set `CODEX_THREAD_ID` to the id of any existing thread, and the receiver then sees that thread as the sender. relay is a local tool and does not close this gap.
- The local endpoint grants full control to SYSTEM, Administrators and the user running the server, and read and write to the accounts `CodexSandboxOffline` and `CodexSandboxOnline` when they exist (the accounts commands run as inside a Codex sandbox). Other accounts cannot connect. Any process that can connect and passes the caller check can send to any target; there is no per-target permission.

### Targets

- `claude <session id>`: the live session with that id, looked up at every send. A session file counts only if its process is alive with the start time the file recorded. No live file fails as `not reachable`; two or more fail as `ambiguous`, naming the pids.
- `codex <thread id>`: that thread. A send to an id never creates a thread.
- `codex <name> <cwd>`: the thread with that exact name whose working directory matches `<cwd>`, created if none exists. `<cwd>` must be an absolute path. Matching ignores case, treats `\` and `/` alike and ignores a trailing separator. Two or more matches fail as `ambiguous`, naming the thread ids.

Sending to your own address is refused. Sends to the same target are delivered one at a time; the order of two near-simultaneous sends is not guaranteed.

### Delivery into Codex

relay resumes the thread (the daemon loads it if it was unloaded) and acts on its status:

- idle: a new turn starts with the message.
- active: the message is steered into the running turn (Codex's `turn/steer`).
- active but the turn cannot be steered: the message is queued behind the turn.
- system error: the send fails.

If the steer fails because the turn ended or was replaced, relay resumes again. If the same turn is still running it queues the message; otherwise it acts on the new status once more, and a second steer failure fails the send as `target busy, retry`.

When relay creates a thread it starts it in `<cwd>`, with the model, sandbox and approval policy taken from the user's Codex configuration, delivers the message as the first turn, waits for the daemon to announce that the turn started, and then sets the thread's name. The name comes last because a named thread with no turn would stay on disk unlisted, where a later by-name send could not find it. If naming fails, or the announcement does not come within 30 seconds, the send is `delivered` with a note naming the unnamed thread's id.

After every delivery relay unsubscribes from the thread so the daemon can unload it. While subscribed, the daemon may send relay requests meant for the thread's user; relay never answers one, because its answer would be taken as the user's.

### Delivery into Claude Code

Claude Code keeps two files per running session in `~/.claude/sessions/`. The session file `<pid>.json` records the session id, the process start time, and `messagingSocketPath`, the named pipe the session listens on for messages (its inbox pipe). The key file `<pid>.<hash>.key` (the hash is the SHA-256 of the lowercased inbox pipe path) holds a `peerToken`, which relay presents as the first line it writes to the inbox pipe. relay opens the inbox pipe, writes the token, then writes the message. The send fails, naming the pid or the file, when the key file is missing or unreadable, or when its start time or pid domain disagrees with the session file. relay reads nothing back from the pipe, so `delivered` means the message was written.

### Results

- `delivered`: the target accepted the handoff (the daemon accepted the turn, steer or queue call; or the message was written to the Claude Code inbox pipe). What the target does with the message afterward is not relay's concern.
- `failed`: nothing landed.
- `may not have landed`: the handoff was sent but its answer never came (timeout, dropped connection). The message may or may not have arrived. relay never resends.

Timeouts: 120 seconds for each call to the daemon (150 seconds to connect to it, which includes the first call), 30 seconds for the daemon to announce a started turn when relay creates a thread, 10 seconds to open or write a Claude Code inbox pipe, and 1020 seconds for the client to wait for the server.

## Agent skills

The repository ships one skill per agent, `skills/claude/relay/SKILL.md` and `skills/codex/relay/SKILL.md`: short instructions that tell the agent how to invoke `relay send` and how to answer a received message. The Claude Code skill covers sending to Codex threads; for other Claude Code sessions it points to SendMessage. The Codex skill covers sending to both Claude Code sessions and Codex threads. The skill texts are built into the binary as templates; `relay install` writes them into each agent's skill folder with the full path of the running `relay` binary in every command line (the binary need not be on `PATH`), and `relay uninstall` removes them (see [docs/USAGE.md](docs/USAGE.md)). A running Codex daemon loads skills only when it starts; `relay install` does not restart it, so run `codex app-server daemon restart` afterwards to make it pick up the new Codex skill.

## Logs

The server logs to `relay/relay.log` under the platform data directory (`%APPDATA%\relay\relay.log` on Windows). The log rotates at 10 MB and keeps 3 rotated files. Every send is logged with its message id, caller, target and outcome.
