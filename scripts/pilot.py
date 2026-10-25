"""Bounded, read-only smoke pilot. Never print endpoint/token values or raw transport errors."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys
import threading
import time

ROOT = Path(__file__).resolve().parent.parent
KEYS = ("YELLOWSTONE_A_URL", "YELLOWSTONE_B_URL")


def preflight():
    urls = [os.environ.get(key, "") for key in KEYS]
    names = [os.environ.get(f"YELLOWSTONE_{who}_NAME", "") for who in "AB"]
    checks = {
        "endpoints_configured": all(urls),
        "distinct_endpoints": bool(all(urls) and urls[0] != urls[1]),
        "https_endpoints": all(url.startswith("https://") for url in urls),
        "provider_names_configured": all(names),
        "distinct_provider_names": bool(all(names) and names[0] != names[1]),
        "owner_mini": platform.node().split(".")[0].lower() in ("ironman", "apsarth"),
    }
    return {"kind": "preflight", "checks": checks, "ready": all(checks.values()),
            "limit": "Configuration checks do not establish provider independence or protocol compatibility."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--preflight", action="store_true")
    mode.add_argument("--run", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "pilot-results")
    args = parser.parse_args()
    check = preflight()
    print(json.dumps(check), flush=True)
    if args.preflight:
        return 0
    if not check["ready"]:
        print("Pilot not started: complete preflight checks on an owner Mini.", file=sys.stderr)
        return 2
    # Bound inputs before building or making any connection. Do not echo invalid values.
    try:
        blocks = int(os.environ.get("RECONCILE_MAX_BLOCKS", "100"))
        seconds = int(os.environ.get("RECONCILE_MAX_SECONDS", "300"))
        # 1200 s: the wire guard's 8192-bank limit ends a connection after about 1,737 s at 212 ms
        # per slot, and no rotation exists yet. A longer run would fail for that reason alone.
        assert 1 <= blocks <= 1000 and 1 <= seconds <= 1200
    except (ValueError, AssertionError):
        print("Invalid pilot bounds: blocks 1..1000; seconds 1..1200.", file=sys.stderr)
        return 2
    # Build scripts do not need provider credentials. Keep build output separate from pilot evidence.
    build_env = {key: value for key, value in os.environ.items()
                 if not key.startswith("YELLOWSTONE_")}
    subprocess.run(["cargo", "build", "--locked", "--release", "--features", "grpc",
                    "--example", "two_providers"], cwd=ROOT, env=build_env, check=True)
    # Exclusive directory: never overwrite a previous measurement.
    output = args.output / time.strftime("%Y%m%d-%H%M%S", time.gmtime())
    output.mkdir(parents=True, exist_ok=False)
    manifest = subprocess.check_output([sys.executable, "scripts/source_manifest.py"], cwd=ROOT)
    (output / "source-manifest.sha256").write_bytes(manifest)
    started = time.monotonic()
    child = subprocess.Popen([str(ROOT / "target/release/examples/two_providers")], cwd=ROOT,
                             stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    peak_rss_kib = [0]
    stop = threading.Event()

    def sample_memory():
        while not stop.wait(0.25):
            try:
                rss = subprocess.check_output(["ps", "-o", "rss=", "-p", str(child.pid)],
                                              stderr=subprocess.DEVNULL, text=True)
                peak_rss_kib[0] = max(peak_rss_kib[0], int(rss.strip()))
            except (subprocess.CalledProcessError, ValueError):
                pass

    sampler = threading.Thread(target=sample_memory, daemon=True)
    sampler.start()
    watchdog = threading.Timer(seconds + 15, child.kill)
    watchdog.start()
    summary = None
    emitted = 0
    harness_error = None
    try:
        with (output / "events.jsonl").open("x") as stream:
            for line in child.stdout:
                # Child output is data. Never echo an unparseable line: it could carry anything.
                try:
                    event = json.loads(line)
                    if event.get("kind") not in ("emitted", "summary"):
                        raise ValueError
                except (ValueError, AttributeError):
                    harness_error = "child produced output that is not a pilot event"
                    child.kill()
                    break
                stream.write(json.dumps(event) + "\n")
                stream.flush()
                if event["kind"] == "emitted":
                    emitted += 1
                else:
                    summary = event
        code = child.wait()
    finally:
        if child.poll() is None:
            child.kill()
        child.wait()
        watchdog.cancel()
        stop.set()
        sampler.join()
    passed = (harness_error is None and code == 0 and bool(summary and summary.get("target_reached"))
              and emitted == blocks)
    # A failed run states why. The child's summary carries its own sanitized failure message.
    failure = None if passed else (
        harness_error or (summary or {}).get("failure") or "child ended without a summary")
    report = {"kind": "pilot_result", "passed": passed, "failure": failure, "exit_code": code,
              "emitted_records": emitted, "target_blocks": blocks,
              "deadline_seconds": seconds, "elapsed_seconds": time.monotonic() - started,
              "sampled_peak_process_rss_kib": peak_rss_kib[0],
              "manifest_sha256": hashlib.sha256(manifest).hexdigest(),
              "summary": summary,
              "limits": ["Smoke pilot only; not a soak, crash-recovery or outage test.",
                         "RSS sampled every 250ms; includes upstream and transport allocations.",
                         "No durable live payload sink or payload equality verification."]}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
