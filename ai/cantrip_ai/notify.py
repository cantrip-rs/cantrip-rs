"""Telling someone what a training run did (docs/ai-training.md, "Watching a run"): a stage that failed, a run that
stalled or whose driver died, a warning a report flagged, a gate's verdict, a generation finished, a loop held.

Where a notification goes is this machine's own business and never the repository's: a hook command and/or a webhook,
named in the environment or in a file outside the repository.

- `CANTRIP_AI_NOTIFY_HOOK`: a command line, run through the shell for each notification with the message in
  `CANTRIP_AI_MESSAGE`, the event in `CANTRIP_AI_EVENT`, and the whole payload as JSON on its standard input.
- `CANTRIP_AI_NOTIFY_WEBHOOK`: a URL the payload is POSTed to as JSON with a `content` field holding the message (a
  Discord webhook takes it as it is; so does anything that reads `content`).
- `CANTRIP_AI_NOTIFY`: a JSON file `{"hook": ..., "webhook": ..., "events": [...]}` (default: `notify.json` in
  `%LOCALAPPDATA%\\cantrip-ai` on Windows, `~/.config/cantrip-ai` elsewhere); the two variables above win over it.
  `events`, when given, names the only events sent (see `EVENTS`).

Every notification is also appended to `ROOT/notifications.jsonl`, sent anywhere or not, with what each channel
answered. Sending never raises and never holds a run up for more than `TIMEOUT` seconds a channel.

    python -m cantrip_ai.notify --root ROOT --test "hello"     # send one, and say where it went

No torch and no numpy: the driver and the supervisor import this.
"""

from __future__ import annotations

import argparse
import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

ENV_HOOK = "CANTRIP_AI_NOTIFY_HOOK"
ENV_WEBHOOK = "CANTRIP_AI_NOTIFY_WEBHOOK"
ENV_FILE = "CANTRIP_AI_NOTIFY"
LOG = "notifications.jsonl"
TIMEOUT = 30.0
# What is told: `stage-failed` (a stage ended a generation), `measurement-failed` (one failed and was passed over),
# `data-failed` (a data-producing stage failed and was allowed to), `warning` (a report's flag, a preflight's doubt),
# `recovered` (a known failure the driver got past by itself), `verdict` (the gate's), `finished` (a generation),
# `stalled`, `dead` (the supervisor's, of a driver), `retry`, `held`, `stopped`, `resumed` (the loop's own), `test`.
EVENTS = ("stage-failed", "measurement-failed", "data-failed", "warning", "recovered", "verdict", "finished", "stalled",
          "dead", "retry", "held", "stopped", "resumed", "test")


def config_file() -> Path:
    if given := os.environ.get(ENV_FILE):
        return Path(given)
    if os.name == "nt" and os.environ.get("LOCALAPPDATA"):
        return Path(os.environ["LOCALAPPDATA"]) / "cantrip-ai" / "notify.json"
    return Path.home() / ".config" / "cantrip-ai" / "notify.json"


def config() -> dict:
    """Where notifications go on this machine: the file's `hook`, `webhook` and `events`, the environment's over it."""
    out: dict = {}
    try:
        read = json.loads(config_file().read_text(encoding="utf-8"))
        if isinstance(read, dict):
            out = {k: read[k] for k in ("hook", "webhook", "events") if read.get(k)}
    except (OSError, ValueError):
        pass
    if os.environ.get(ENV_HOOK):
        out["hook"] = os.environ[ENV_HOOK]
    if os.environ.get(ENV_WEBHOOK):
        out["webhook"] = os.environ[ENV_WEBHOOK]
    return out


def payload(event: str, message: str, root=None, generation=None, stage=None, **details) -> dict:
    return dict(event=event, message=message, root=str(root) if root is not None else None, generation=generation,
                stage=stage, host=socket.gethostname(), at=time.strftime("%Y-%m-%d %H:%M:%S"), details=details)


def _kill_tree(p):
    """The hook and everything it started, ended."""
    try:
        if os.name == "nt":
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(p.pid)], capture_output=True, timeout=30)
        else:
            import signal  # noqa: PLC0415

            os.killpg(p.pid, signal.SIGKILL)
    except (OSError, subprocess.SubprocessError):
        pass
    try:
        p.kill()
    except OSError:
        pass


def _hook(command: str, body: dict, timeout: float | None = None) -> str:
    """The hook run through the shell, in a process group of its own, its input and output files rather than pipes:
    the time limit is then the hook's own and holds whatever it started - a child it left behind holding a pipe would
    otherwise keep the run waiting past it - and at the limit the whole tree is killed."""
    timeout = TIMEOUT if timeout is None else timeout
    env = {**os.environ, "CANTRIP_AI_MESSAGE": str(body["message"]), "CANTRIP_AI_EVENT": str(body["event"])}
    group = dict(creationflags=subprocess.CREATE_NEW_PROCESS_GROUP) if os.name == "nt" else dict(start_new_session=True)
    with tempfile.TemporaryFile() as said, tempfile.TemporaryFile() as given:
        given.write(json.dumps(body, default=str).encode("utf-8"))
        given.seek(0)
        p = subprocess.Popen(command, shell=True, stdin=given, stdout=said, stderr=subprocess.STDOUT, env=env, **group)
        try:
            code = p.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            _kill_tree(p)
            return f"failed: no answer in {timeout:.0f} s; killed"
        said.seek(0)
        text = said.read().decode("utf-8", errors="replace")
    return "ok" if code == 0 else f"exit {code}: {text.strip()[-200:]}"


def _webhook(url: str, body: dict) -> str:
    where = " ".join(str(x) for x in (f"gen{body['generation']}" if body.get("generation") is not None else "",
                                      body.get("stage") or "") if x)
    content = f"**[{body['event']}]** {body['host']}{' ' + where if where else ''}: {body['message']}"
    data = json.dumps(dict(body, content=content[:1900]), default=str).encode("utf-8")
    request = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json",
                                                              "User-Agent": "cantrip-ai-notify"})
    with urllib.request.urlopen(request, timeout=TIMEOUT) as answer:  # noqa: S310 (the URL is this machine's own)
        return f"ok ({answer.status})"


def send(event: str, message: str, root=None, generation=None, stage=None, log=None, **details) -> dict:
    """One notification, to the hook and the webhook this machine names (neither: only the log under `root`). Returns
    what each channel answered; nothing here raises, whatever the details hold or the configuration says."""
    sent: dict = {}
    try:
        _send(event, message, root, generation, stage, log, details, sent)
    except Exception as e:  # noqa: BLE001 (a notification never fails a run)
        sent.setdefault("error", f"failed: {type(e).__name__}: {e}")
    return sent


def _send(event, message, root, generation, stage, log, details: dict, sent: dict):
    body = payload(event, str(message), root, generation, stage, **details)
    cfg = config()
    if not cfg.get("events") or event in cfg["events"]:
        for name, call in (("hook", _hook), ("webhook", _webhook)):
            if cfg.get(name):
                try:
                    sent[name] = call(cfg[name], body)
                except Exception as e:  # noqa: BLE001 (a notification never fails a run)
                    sent[name] = f"failed: {type(e).__name__}: {e}"
    if root is not None:
        try:
            with (Path(root) / LOG).open("a", encoding="utf-8") as f:
                f.write(json.dumps(dict(body, sent=sent), default=str) + "\n")
        except OSError:
            pass
    if log is not None:
        try:
            log(f"notify [{event}] {message}" + (f" ({', '.join(f'{k}: {v}' for k, v in sent.items())})" if sent else
                                                 " (no hook or webhook configured: logged only)"))
        except Exception:  # noqa: BLE001
            pass


def main(argv=None):
    a = argparse.ArgumentParser(description="Send a test notification, and say where notifications go here.")
    a.add_argument("--root", help="the ai-data directory (its notifications.jsonl gets the line)")
    a.add_argument("--test", metavar="MESSAGE", default="a test notification")
    args = a.parse_args(argv)
    cfg = config()
    print(f"config file: {config_file()} ({'found' if config_file().exists() else 'none'})")
    print(f"hook: {'set' if cfg.get('hook') else 'none'}; webhook: {'set' if cfg.get('webhook') else 'none'}"
          + (f"; only {', '.join(cfg['events'])}" if cfg.get("events") else ""))
    sent = send("test", args.test, root=args.root)
    print(json.dumps(sent))
    return 0 if all(str(v).startswith("ok") for v in sent.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
