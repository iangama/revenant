#!/usr/bin/env python3
"""Read only recent usage telemetry for this Codex thread; never print chat data."""
from datetime import datetime, timezone
import json
import math
import os
from pathlib import Path


def check():
    thread = os.environ.get("CODEX_THREAD_ID", "")
    if not thread:
        return {"status": "unavailable", "reason": "CODEX_THREAD_ID not provided"}
    root = Path(os.environ.get("CODEX_HOME", str(Path.home() / ".codex")))
    paths = list((root / "sessions").rglob(f"*{thread}*.jsonl"))
    if not paths:
        return {"status": "unavailable", "reason": "current thread log not found"}
    path = max(paths, key=lambda p: p.stat().st_mtime)
    with path.open("rb") as stream:
        stream.seek(max(0, path.stat().st_size - 2 * 1024 * 1024))
        lines = stream.read().splitlines()
    for line in reversed(lines):
        try:
            entry = json.loads(line)
        except (ValueError, UnicodeDecodeError):
            continue
        payload = entry.get("payload", {})
        if entry.get("type") != "event_msg" or payload.get("type") != "token_count":
            continue
        limits = payload.get("rate_limits") or {}
        for name in ("primary", "secondary"):
            window = limits.get(name) or {}
            used = window.get("used_percent")
            if window.get("window_minutes") != 300 or not isinstance(used, (float, int)):
                continue
            if not math.isfinite(used) or not 0 <= used <= 100:
                continue
            try:
                observed = datetime.fromisoformat(entry["timestamp"].replace("Z", "+00:00"))
                now = datetime.now(timezone.utc)
                stale = (now - observed).total_seconds() > 600 or window.get("resets_at", 0) <= now.timestamp()
            except (KeyError, TypeError, ValueError):
                continue
            remaining = round(100 - used, 2)
            return {"status": "stale" if stale else "ok", "remaining_percent": remaining,
                    "window_minutes": 300, "observed_at": entry["timestamp"],
                    "summary_due": remaining <= 8 and not stale}
    return {"status": "unavailable", "reason": "no recent five-hour usage event"}


if __name__ == "__main__":
    try:
        print(json.dumps(check()))
    except OSError:
        print(json.dumps({"status": "unavailable", "reason": "local telemetry cannot be read"}))
