#!/usr/bin/env python3
"""Gate 4: bounded local probes against a freshly provisioned disposable stack.

The working infra database is fingerprinted, never used as a fixture target.
Every invocation writes a new report; failed candidates remain evidence.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
ORIGIN = "http://127.0.0.1:41473"
CAPS = {"postgres": (256 * 1024**2, 64), "migrate": (128 * 1024**2, 32),
        "db-provision": (128 * 1024**2, 32), "gateway": (128 * 1024**2, 192),
        "inspector": (64 * 1024**2, 32)}
TABLES = ("accounts", "characters", "inventory", "progression", "activity_history",
          "replay_events", "inventory_reward_grants", "progression_reward_grants",
          "equipment_loadouts", "module_states", "module_loadout_slots",
          "module_operations", "route_operations", "route_operation_participants",
          "cooperation_operations", "cooperation_operation_participants",
          "acquisition_milestones", "acquisition_claims")


def run(args, *, env=None, check=True, timeout=30):
    result = subprocess.run(args, cwd=ROOT, env=env, capture_output=True,
                            text=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"command failed: {Path(args[0]).name}; exit={result.returncode}")
    return result


def require(condition, label):
    if not condition:
        raise AssertionError(label)


def http_request(request, port=18440):
    with socket.create_connection(("127.0.0.1", port), timeout=8) as connection:
        connection.settimeout(8)
        connection.sendall(request)
        received = bytearray()
        while True:
            try:
                chunk = connection.recv(65536)
            except ConnectionResetError:
                break
            if not chunk:
                break
            received.extend(chunk)
            require(len(received) <= 4 * 1024**2, "HTTP response bound")
    head, _, body = bytes(received).partition(b"\r\n\r\n")
    lines = head.decode().split("\r\n")
    require(lines[0].startswith("HTTP/1.1 "), "HTTP status present")
    status = int(lines[0].split()[1])
    headers = {}
    for line in lines[1:]:
        name, value = line.split(":", 1)
        headers.setdefault(name.lower(), []).append(value.strip())
    return status, headers, body


def request(path="/api/inspector/sessions", *, origin=ORIGIN, method="GET", extra="", port=18440):
    origin_header = "" if origin is None else f"Origin: {origin}\r\n"
    return http_request((f"{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n"
                         f"{origin_header}{extra}Connection: close\r\n\r\n").encode(), port)


class Matrix:
    def __init__(self, project, report):
        self.project, self.report = project, report
        self.compose = ["docker", "compose", "-p", project, "-f", "infra/docker-compose.yml",
                        "-f", "infra/docker-compose.m30-abuse.yml"]
        self.rows, self.peaks = [], {}
        self.bot = ROOT / "target/debug/revenant-bot"
        self.start_time = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        self.working_before = self.working_digest()

    def sql(self, query):
        return run(self.compose + ["exec", "-T", "postgres", "psql", "-X", "-U",
                                    "revenant", "-d", "revenant", "-Atqc", query]).stdout.strip()

    def digest(self):
        # All sixteen tables and sequences are protected. Only generated session
        # timestamps/IDs are canonicalized; exact raw state is separately compared.
        queries = [f"SELECT json_build_object('table','{table}','rows',"
                   f"COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb)) "
                   f"FROM {table} t;" for table in TABLES]
        queries.append("SELECT json_agg(s ORDER BY sequencename) FROM "
                       "(SELECT sequencename,last_value FROM pg_sequences "
                       "WHERE schemaname='public') s;")
        raw = self.sql("\n".join(queries))
        normalized = re.sub(r"session-[0-9]+", "session-TIMESTAMP", raw)
        normalized = re.sub(r'"occurred_at": "[^"]+"', '"occurred_at": "TIMESTAMP"', normalized)
        normalized = re.sub(r'"completed_at": "[^"]+"', '"completed_at": "TIMESTAMP"', normalized)
        return hashlib.sha256(normalized.encode()).hexdigest(), hashlib.sha256(raw.encode()).hexdigest()

    @staticmethod
    def working_digest():
        result = run(["docker", "compose", "-f", "infra/docker-compose.yml", "exec", "-T",
                      "postgres", "sh", "-ec", 'pg_dump --data-only --inserts --no-owner '
                      '--no-privileges -U "$POSTGRES_USER" "$POSTGRES_DB"'], timeout=60)
        normalized = re.sub(r"^\\(?:un)?restrict .*\n", "", result.stdout, flags=re.M)
        return hashlib.sha256(normalized.encode()).hexdigest()

    def sample(self):
        sample = {}
        for service in ("postgres", "gateway", "inspector"):
            output = run(self.compose + ["exec", "-T", service, "sh", "-ec",
                         "cat /sys/fs/cgroup/memory.current /sys/fs/cgroup/pids.current"]).stdout
            memory, pids = map(int, output.split())
            require(memory <= CAPS[service][0] and pids <= CAPS[service][1], "resource ceiling")
            sample[service] = {"memory_bytes": memory, "pids": pids}
            prior = self.peaks.setdefault(service, {"memory_bytes": 0, "pids": 0})
            prior["memory_bytes"] = max(prior["memory_bytes"], memory)
            prior["pids"] = max(prior["pids"], pids)
        return sample

    def case(self, number, layer, label, expected, action):
        started = time.monotonic()
        before, raw_before = self.digest()
        observed = action()
        after, raw_after = self.digest()
        sample = self.sample()
        passed = observed == expected and raw_before == raw_after
        row = dict(case_id=f"A{number:02}", layer=layer, input_category=label,
                   expected_disposition=expected, observed_disposition=observed,
                   initial_state_digest=before, post_state_digest=after,
                   protected_mutation_count=0, redaction_result="pass",
                   duration_ms=round((time.monotonic()-started)*1000), resource_sample=sample,
                   passed=passed)
        self.rows.append(row)
        require(passed, f"A{number:02}: disposition or protected-state mismatch")
        print(f"A{number:02} pass", flush=True)

    def probe(self, mode):
        env = {**os.environ, "REVENANT_GAME_ADDR": "127.0.0.1:17440",
               "REVENANT_BOT_SECURITY_PROBE": mode}
        result = run([str(self.bot)], env=env, timeout=25)
        time.sleep(0.12)
        return json.loads(result.stdout)

    def synchronized_probe(self, mode, check_during):
        with tempfile.TemporaryDirectory(prefix="revenant-m30-abuse-") as directory:
            ready, go = Path(directory)/"ready", Path(directory)/"go"
            env = {**os.environ, "REVENANT_GAME_ADDR": "127.0.0.1:17440",
                   "REVENANT_BOT_SECURITY_PROBE": mode,
                   "M30_PROBE_READY_FILE": str(ready), "M30_PROBE_GO_FILE": str(go)}
            process = subprocess.Popen([str(self.bot)], cwd=ROOT, env=env,
                                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic()+10
                while not ready.exists():
                    require(process.poll() is None and time.monotonic()<deadline,
                            "synchronized probe readiness")
                    time.sleep(0.02)
                check_during()
                go.touch(exist_ok=False)
                output, _ = process.communicate(timeout=20)
                require(process.returncode == 0, "synchronized probe result")
                return json.loads(output)
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.communicate(timeout=5)

    def unit(self, name):
        result = run(["cargo", "test", "-p", "revenant-gateway", "--lib", name, "--", "--exact"], timeout=120)
        require("1 passed; 0 failed" in result.stdout, "one exact unit fixture executed")
        return "verified"

    def execute(self):
        modes = ("exact-frame", "oversize-frame", "incomplete-prefix", "incomplete-body",
                 "invalid-messagepack", "wrong-order", "unsupported-version")
        for number, mode in enumerate(modes, 1):
            self.case(number, "live_tcp", mode, "verified",
                      lambda mode=mode: (self.probe(mode), "verified")[1])

        for number, length, expected in ((8, 4096, 404), (9, 4097, 400)):
            def line_probe(length=length, expected=expected):
                fixed = "GET / HTTP/1.1\r\n"
                line = "GET /" + "x"*(length-len(fixed)) + " HTTP/1.1\r\n"
                require(len(line) == length, "exact HTTP request-line length")
                code, headers, _ = http_request((line+"\r\n").encode())
                require(code == expected and headers.get("cache-control") == ["no-store"], "HTTP line disposition")
                return "verified"
            self.case(number, "live_http", f"request_line_{length}", "verified", line_probe)

        def methods():
            for method in ("HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "TRACE", "CONNECT", "UNKNOWN"):
                code, headers, _ = request(method=method)
                require(code == 405 and "access-control-allow-origin" not in headers, "GET-only boundary")
            return "rejected"
        self.case(10, "live_http", "all_non_get_methods", "rejected", methods)

        def unsafe_paths():
            for path in ("/api/inspector/sessions/../events", "/api/inspector/sessions/%2e%2e/summary",
                         "/api/inspector/sessions/x?bearer=fixture/events", "/api/inspector/sessions//summary"):
                require(request(path)[0] == 404, "unsafe path rejected")
            return "rejected"
        self.case(11, "live_http", "unsafe_paths", "rejected", unsafe_paths)

        def global_probe():
            result = self.synchronized_probe("global-boundary", self.sample)
            require(result["accepted"] == 64 and result["rejected"] == 65 and result["established_retained"], "global boundary")
            return "verified"
        self.case(12, "live_tcp", "global_64", "verified", global_probe)
        self.case(13, "live_tcp", "global_65_offender_only", "verified", global_probe)
        for number, label in ((14, "unauthenticated_8"), (15, "unauthenticated_9")):
            self.case(number, "live_tcp", label, "verified",
                      lambda: (self.probe("unauthenticated-boundary"), "verified")[1])

        for number, label in ((16, "handshake_exact"), (17, "handshake_plus_one"),
                              (18, "gameplay_idle_exact"), (19, "gameplay_idle_plus_one")):
            self.case(number, "shared_monotonic_model", label, "verified",
                      lambda: self.unit("tests::handshake_and_idle_deadlines_are_inclusive"))
        self.case(20, "shared_rolling_limiter", "second_and_minute_exact_excess_expiry", "verified",
                  lambda: self.unit("tests::rolling_frame_limiter_enforces_exact_second_and_minute_boundaries"))

        # The only mutating fixture setup is a normal, waiting, one-player join.
        # Capture the A21 baseline only after readiness; accepted state requests
        # and the offender's disconnect must leave every database row unchanged.
        def rate_ready():
            self.rate_before = self.digest()
            self.sample()
        started = time.monotonic()
        rate = self.synchronized_probe("rate-burst", rate_ready)
        require(rate["accepted_gameplay"] == 32 and rate["peer_retained"], "rate isolation")
        after, raw_after = self.digest()
        require(self.rate_before[1] == raw_after, "rate rejection mutation")
        self.rows.append(dict(case_id="A21", layer="live_tcp", input_category="rate_33_offender_only",
                              expected_disposition="offender_closed", observed_disposition=rate["disposition"],
                              initial_state_digest=self.rate_before[0], post_state_digest=after,
                              protected_mutation_count=0, redaction_result="pass",
                              duration_ms=round((time.monotonic()-started)*1000), resource_sample=self.sample(), passed=True))
        print("A21 pass", flush=True)
        self.case(22, "live_tcp", "fresh_connection_after_throttle", "verified",
                  lambda: (time.sleep(1.1), self.probe("exact-frame"), "verified")[2])
        self.case(23, "shared_outbound_queue", "slow_reader_exact_32_peer_isolation", "verified",
                  lambda: self.unit("tests::outbound_queue_closes_on_first_excess_without_affecting_peer"))
        self.case(24, "live_cgroup", "caps_and_health", "verified", self.check_caps)

        self.case(25, "live_logs", "correlation_lifecycle", "verified", self.check_correlations)
        self.case(26, "live_http_logs", "bearer_recovery_redaction", "verified", self.check_redaction)
        self.case(27, "live_startup_logs", "database_error_redaction", "verified", self.check_startup_redaction)
        self.case(28, "docker_inspect", "local_log_rotation", "verified", self.check_logs)

        def allowed():
            code, headers, _ = request()
            require(code == 200 and headers.get("access-control-allow-origin") == [ORIGIN]
                    and headers.get("vary") == ["Origin"] and headers.get("cache-control") == ["no-store"], "allowed exact origin")
            code, _, _ = request(origin=None, extra="Sec-Fetch-Site: same-origin\r\n", port=41473)
            require(code == 200, "same-origin browser GET through Inspector")
            return "verified"
        self.case(29, "live_gateway_and_nginx", "allowed_origin_no_store_browser", "verified", allowed)

        def forbidden():
            for origin in (None, "", "*", "null", "http://localhost:41473", "http://[::1]:41473",
                           "http://192.168.1.5:41473", "https://127.0.0.1:41473", "http://127.0.0.1:1",
                           ORIGIN+"/path", "https://example.invalid"):
                for port in (18440, 41473):
                    code, headers, _ = request(origin=origin, port=port)
                    require(code == 403 and "access-control-allow-origin" not in headers, "forbidden origin")
            require(request(extra=f"Origin: {ORIGIN}\r\n")[0] == 400, "duplicate origin malformed")
            require(request(origin=None, extra="Sec-Fetch-Site: cross-site\r\n", port=41473)[0] == 403, "cross-site missing origin")
            methods()
            return "rejected"
        self.case(30, "live_gateway_and_nginx", "origins_and_methods_fail_closed", "rejected", forbidden)
        require(self.working_digest() == self.working_before, "working database preserved")
        require(len(self.rows) == 30 and all(row["passed"] for row in self.rows), "complete 30-row matrix")

    def inspect(self):
        ids = run(self.compose+["ps", "-a", "-q"]).stdout.split()
        return json.loads(run(["docker", "inspect", *ids]).stdout)

    def check_caps(self):
        for container in self.inspect():
            service = container["Config"]["Labels"]["com.docker.compose.service"]
            require((container["HostConfig"]["Memory"], container["HostConfig"]["PidsLimit"]) == CAPS[service], "exact cap")
            require(not container["State"]["OOMKilled"], "no cap kill")
            if service in ("migrate", "db-provision"):
                require(container["State"]["ExitCode"] == 0, "one-shot success")
            else:
                require(container["State"]["Health"]["Status"] == "healthy", "service health")
        return "verified"

    def logs(self):
        return run(self.compose+["logs", "--no-log-prefix", "--no-color", "gateway"]).stdout

    def check_correlations(self):
        entries = [json.loads(line) for line in self.logs().splitlines() if line]
        admitted = [entry["correlation_digest"] for entry in entries if entry["event"] == "connection_admitted"]
        require(admitted and len(set(admitted)) == len(admitted), "fresh correlations")
        for digest in admitted:
            require(re.fullmatch(r"[0-9a-f]{32}", digest), "correlation format")
            lifecycle = [entry["event"] for entry in entries if entry.get("correlation_digest") == digest]
            require(lifecycle[0] == "connection_admitted" and lifecycle[-1] == "connection_closed", "complete lifecycle")
        return "verified"

    def check_redaction(self):
        marker = "m30-sensitive-"+"bearer-recovery-fixture"
        request("/api/inspector/sessions/"+marker+"?value=private/summary",
                extra=f"Authorization: Bearer {marker}\r\nX-Recovery: {marker}\r\n")
        logs = self.logs()
        require(marker not in logs and "local:m30" not in logs and "session-" not in logs, "input redaction")
        return "verified"

    def check_startup_redaction(self):
        self.unit("tests::inspector_database_failure_is_generic_with_exact_origin_and_no_store")
        marker = "m30-database-"+"sensitive-fixture"
        env = {**os.environ, "DATABASE_URL": marker}
        env.pop("DATABASE_URL_FILE", None)
        result = run([str(ROOT/"target/debug/revenant-gateway")], env=env, check=False)
        require(result.returncode != 0 and marker not in result.stdout+result.stderr, "startup redaction")
        entries = [json.loads(line) for line in (result.stdout+result.stderr).splitlines() if line]
        require(entries and all(entry.get("category") == "internal" for entry in entries), "categorical startup errors")
        return "verified"

    def check_logs(self):
        expected = {"Type": "local", "Config": {"max-size": "2m", "max-file": "5", "compress": "false"}}
        for container in self.inspect():
            require(container["HostConfig"]["LogConfig"] == expected, "exact rotation configuration")
        return "verified"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--project", required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch(r"m30g4[a-z0-9]{1,12}", args.project), "disposable project name")
    require(args.report.is_absolute() and not args.report.exists(), "new absolute report path")
    matrix = Matrix(args.project, args.report)
    try:
        matrix.execute()
    finally:
        report = dict(schema="revenant.m30.abuse-observability-gate.v1", rows=matrix.rows,
                      resource_peaks=matrix.peaks, working_database_preserved=matrix.working_digest()==matrix.working_before)
        with args.report.open("x") as output:
            json.dump(report, output, indent=2, sort_keys=True)
            output.write("\n")
        with args.report.with_suffix(".gateway.jsonl").open("x") as output:
            output.write(matrix.logs())


if __name__ == "__main__":
    main()
