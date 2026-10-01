---
name: relay
description: "Use to send a message to a Codex thread with relay, or to answer a message that arrived as a `<cross-session-message ... from=\"codex ...\">` block. For other Claude Code sessions use SendMessage, not relay."
---

# relay

relay hands a text message to a Codex thread. To reach another Claude Code session use your own SendMessage; do not use relay for that.

## Send

```
@RELAY@ send codex <thread id> "<message>"
@RELAY@ send codex <name> <cwd> "<message>"
```

- `<thread id>` is an existing thread. It never creates one.
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

A message from Codex arrives as:

```
<cross-session-message id="<id>" from="codex <thread id>">
<text>
</cross-session-message>
```

Answer with the same command and the `from` address: `@RELAY@ send codex <thread id> "<reply>"`.
