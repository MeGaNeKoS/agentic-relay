---
name: relay
description: "Use to send a message to a Claude Code session or another Codex thread with relay, or to answer a message that arrived as a `<cross-session-message ... from=\"...\">` block."
---

# relay

relay hands a text message to a Claude Code session or a Codex thread.

## Send

```
@RELAY@ send claude <session id> "<message>"
@RELAY@ send codex <thread id> "<message>"
@RELAY@ send codex <name> <cwd> "<message>"
```

- `<session id>` and `<thread id>` name an existing session or thread. Neither creates one.
- `<name> <cwd>` is the thread with that exact name in that directory, created if none exists. `<cwd>` must be an absolute path.
- Quote the message as one argument.

## Result

| Output | Exit | Meaning |
|---|---|---|
| `delivered <id>` | 0 | Handed to the target. |
| `failed: <reason>` | 1 | Nothing landed. |
| `may not have landed ...` | 2 | The answer never came. relay never resends. |

`Error: ...` (exit 1) means the request was not made, for example relay is not running.

relay gives the sender no reply. The target answers by sending its own message.

## Reply

A received message looks like:

```
<cross-session-message id="<id>" from="claude <session id>">
<text>
</cross-session-message>
```

The `from` value is the sender's address. Answer with the same command and that address: `@RELAY@ send claude <session id> "<reply>"`, or `@RELAY@ send codex <thread id> "<reply>"` when `from` starts with `codex`.
