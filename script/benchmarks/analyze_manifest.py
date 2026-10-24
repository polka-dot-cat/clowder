"""Summarize a manifest benchmark and optionally confirm it without rebuilding."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import random
import statistics
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("results", type=Path)
parser.add_argument("--confirm", action="store_true")
args = parser.parse_args()
report = json.loads(args.results.read_text())
root = args.results.resolve().parent
rng = random.Random(report["metadata"]["seed"])
env = os.environ.copy()
for key in list(env):
    if key.startswith("CLOWDER_BENCH_") or key in ("CLOWDER_COMPLETE", "BASH_ENV"):
        env.pop(key)
env.update(GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_SYSTEM=os.devnull, GIT_TERMINAL_PROMPT="0")


def interval(original, cached):
    estimates = []
    for _ in range(10000):
        indexes = [rng.randrange(len(original)) for _ in original]
        estimates.append(statistics.median(original[i] for i in indexes)
                         - statistics.median(cached[i] for i in indexes))
    estimates.sort()
    return [estimates[250], estimates[9749]]


def run(count, mode, command):
    cache = root / "cache" / str(count) / mode
    run_env = dict(env, CLOWDER_BENCH_MODE="validated" if mode == "miss" else mode,
                   CLOWDER_BENCH_CACHE=str(cache))
    if mode == "miss" and cache.exists():
        for path in cache.glob("*.json"):
            path.unlink()
    binary = root / ("clowder-original" if mode == "original" else "clowder-benchmark")
    start = time.perf_counter_ns()
    result = subprocess.run([str(binary), command], cwd=root / "fixtures" / str(count), env=run_env,
                            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    elapsed = (time.perf_counter_ns() - start) / 1e6
    if result.returncode:
        raise RuntimeError(result.stderr.decode())
    return elapsed


def snapshot(directory):
    return {path.name: (path.stat().st_ino, path.stat().st_size, path.stat().st_mtime_ns)
            for path in directory.glob("*.json")}


analysis = {"source_results": args.results.name, "estimator": "difference of medians; paired bootstrap by randomized round",
            "bootstrap_samples": 10000, "confidence": .95, "commands": [], "cache_hit_checks": [],
            "binary_sha256": {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                              for name in ("clowder-original", "clowder-benchmark")}}
for fixture in report["fixtures"]:
    count = fixture["projects"]
    for command, modes in fixture["commands"].items():
        baseline = modes.get("original", modes["baseline"])
        cached = modes["validated"]
        item = {"projects": count, "command": command,
                "baseline_ms": baseline["median_ms"], "cached_ms": cached["median_ms"],
                "saved_ms": baseline["median_ms"] - cached["median_ms"],
                "percent_saved": 100 * (1 - cached["median_ms"] / baseline["median_ms"]),
                "saved_ms_interval": interval(baseline["samples_ms"], cached["samples_ms"])}
        analysis["commands"].append(item)
        print(json.dumps(item), flush=True)
    for mode in ("json", "validated", "stat"):
        before = snapshot(root / "cache" / str(count) / mode)
        assert len(before) == 1, (count, mode, before)
        run(count, mode, "--help")
        assert before == snapshot(root / "cache" / str(count) / mode), (count, mode)
        analysis["cache_hit_checks"].append({"projects": count, "mode": mode, "cache_rewritten": False})

counts = [fixture["projects"] for fixture in report["fixtures"]]
floor = {mode: [] for mode in ("original", "baseline")}
for index in range(report["metadata"]["warmups"] + report["metadata"]["samples"]):
    modes = list(floor)
    rng.shuffle(modes)
    for mode in modes:
        elapsed = run(min(counts), mode, "--version")
        if index >= report["metadata"]["warmups"]:
            floor[mode].append(elapsed)
analysis["version_control"] = {mode: {"median_ms": statistics.median(samples), "samples_ms": samples}
                               for mode, samples in floor.items()}

if args.confirm:
    # Reuse the exact binaries and fixtures after the build and first run have
    # finished. Interleave sizes as well as modes; no rebuild or new source patch.
    cases = [(count, "--help", mode) for count in sorted({min(counts), max(counts)})
             for mode in ("original", "validated", "stat", "miss")]
    cases += [(min(counts), "status", mode) for mode in ("original", "validated")]
    samples = {case: [] for case in cases}
    analysis["confirmation_load_start"] = os.getloadavg()
    for index in range(report["metadata"]["warmups"] + report["metadata"]["samples"]):
        rng.shuffle(cases)
        for count, command, mode in cases:
            elapsed = run(count, mode, command)
            if index >= report["metadata"]["warmups"]:
                samples[(count, command, mode)].append(elapsed)
        if index >= report["metadata"]["warmups"] and (index - report["metadata"]["warmups"] + 1) % 8 == 0:
            print(f"Confirmation: {index - report['metadata']['warmups'] + 1} rounds", flush=True)
    analysis["confirmation"] = []
    for (count, command, mode), values in sorted(samples.items()):
        original = samples[(count, command, "original")]
        analysis["confirmation"].append({"projects": count, "command": command, "mode": mode,
                                         "median_ms": statistics.median(values),
                                         "p95_ms": sorted(values)[math.ceil(len(values) * .95) - 1],
                                         "saved_ms_interval": interval(original, values), "samples_ms": values})
    analysis["confirmation_load_end"] = os.getloadavg()

(root / "analysis.json").write_text(json.dumps(analysis, indent=2) + "\n")
print("Analysis: " + str(root / "analysis.json"), flush=True)
