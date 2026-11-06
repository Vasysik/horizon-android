"""Run the isolated Android workload matrix on connected test devices.

Prepared APKs and disposable world fixtures live outside version control.
The device driver never touches the production application package.
"""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import time

VERSIONS = ("0.7.3", "0.7.7", "0.7.8", "0.7.8-f42ef42")
DEFAULT_VERSIONS = VERSIONS[:3]
NATURAL_CASES = tuple((scene, distance, 0) for scene in
                      ("ocean-flight", "forest-flight", "jungle-flight", "desert-flight",
                       "snow-flight", "nether-flight", "end-flight") for distance in (12,16,24))
CASES = (
    ("lod-off", 12, 0),
    ("lod-off", 16, 0),
    ("lod-off", 24, 0),
    ("nether", 16, 0),
    ("end", 16, 0),
    ("water", 16, 0),
    ("foliage", 16, 0),
    ("sand", 16, 0),
    ("jungle-lab", 16, 0),
    ("entities-control", 16, 0),
    ("entities-static", 16, 0),
    ("entities-tick-control", 16, 0),
    ("entities-ticking", 16, 0),
    ("stream-cold", 24, 0),
    ("stream-warm", 24, 0),
    ("lod-on", 16, 64),
    ("lod-on", 16, 128),
    ("flight-cold", 16, 0),
    ("flight-warm", 16, 0),
    ("camera-orbit", 16, 0),
)
CASES = NATURAL_CASES + CASES


def build_plan(cases, versions, repeats, renderers=('compatibility',)):
    jobs = []
    for repeat in range(repeats):
        for renderer in renderers:
            for index, (scenario, distance, lod) in enumerate(cases):
                order = versions if (repeat + index) % 2 == 0 else tuple(reversed(versions))
                for version in order:
                    jobs.append(dict(key=f'{repeat}-{renderer}-{scenario}-{distance}-{lod}-{version}',
                                     repeat=repeat, scenario=scenario, distance=distance,
                                     lod=lod, version=version, renderer=renderer, status='pending'))
    return jobs


def measurement_samples(rows, events):
    """Use recorder boundaries, excluding loading, menus and partial first windows."""
    bounds = {e["name"]: e.get("report_t_ms") for e in events}
    start, end = bounds.get("measure-start"), bounds.get("measure-end")
    exact = isinstance(start, (int, float)) and isinstance(end, (int, float)) and end > start
    samples = [r for r in rows if r.get("frames") and r.get("context", {}).get("in_world")
               and not r["context"].get("paused")]
    if exact:
        samples = [r for r in samples if start < r.get("t_ms", -1) <= end]
    else:
        samples = samples[-60:-2]
    return samples, exact


def summarize(fixtures, output):
    results = []
    for meta in (fixtures / "cpu-experiments").rglob("run.json"):
        item = json.loads(meta.read_text(encoding="utf8"))
        if item.get("session") != output.parent.name:
            continue
        rows = []
        for report in meta.parent.glob("*.jsonl"):
            for line in report.read_text(encoding="utf8").splitlines():
                try:
                    row = json.loads(line)
                except ValueError:
                    continue
                rows.append(row)
        events_path = meta.parent / "events.json"
        events = json.loads(events_path.read_text()) if events_path.exists() else []
        samples, aligned = measurement_samples(rows, events)
        reasons = []
        validation = meta.parent / "validation.json"
        if validation.exists():
            review = json.loads(validation.read_text(encoding="utf8"))
            if not review.get("valid", False):
                reasons.append(review.get("reason", "visual validation failed"))
        else:
            reasons.append("scene has not been reviewed")
        if not aligned:
            reasons.append("legacy trailing windows; no measurement boundaries")
        if len(samples) < 20:
            reasons.append("insufficient in-world windows")
        if any(r["context"].get("render_distance") != item["distance"] for r in samples):
            reasons.append("render distance mismatch")
        thermal = sorted({r["context"].get("android_power", {}).get("thermal_status", -1) for r in samples})
        if any(t >= 3 for t in thermal if isinstance(t, int)):
            reasons.append("thermal throttling")
        item.update(path=str(meta.parent), valid=not reasons, rejection_reasons=reasons,
                    thermal=thermal, measurement_aligned=aligned, windows=len(samples))
        if samples:
            item["fps"] = statistics.median(r["frames"]["avg_fps"] for r in samples)
            item["fps_definition"] = "median of recorder-window FPS, not total frames / total time"
            seconds = sum(r["frames"].get("seconds", 0) for r in samples)
            item["mean_fps"] = sum(r["frames"].get("frames", 0) for r in samples) / seconds if seconds else None
            for stage in ("cpu", "gpu"):
                values = [r["frames"].get(stage, {}).get("avg_ms") for r in samples]
                values = [v for v in values if isinstance(v, (int, float))]
                item[stage + "_ms"] = statistics.median(values) if values else None
            item["geometry"] = {
                key: statistics.median(r["context"].get("renderer", {}).get(key, 0) for r in samples)
                for key in ("resident_sections", "drawn_sections", "draw_calls", "section_vertex_bytes")
            }
            item["frame_distributions"] = {
                key: statistics.median(values) if values else None
                for key in ("p50_ms", "p95_ms", "p99_ms", "max_ms", "low_1pct_fps", "low_01pct_fps")
                for values in [[r["frames"][key] for r in samples if isinstance(r["frames"].get(key), (int, float))]]
            }
            item["frame_distribution_definition"] = "medians of window percentiles; raw frame samples required for global percentiles"
            stages = sorted({t["stage"] for r in samples for t in r.get("timings", [])})
            item["stages"] = {}
            for stage in stages:
                spans = [t for r in samples for t in r.get("timings", []) if t["stage"] == stage and t.get("calls")]
                wall = [t["wall_ms"] / t["calls"] for t in spans]
                cpu = [t["thread_cpu_ms"] / t["calls"] for t in spans if isinstance(t.get("thread_cpu_ms"), (int, float))]
                item["stages"][stage] = {
                    "wall_ms_per_call": statistics.median(wall) if wall else None,
                    "cpu_ms_per_call": statistics.median(cpu) if cpu else None,
                    "max_wall_ms": max((t["max_ms"] for t in spans), default=None),
                }
        results.append(item)
    output.write_text(json.dumps(results, indent=2), encoding="utf8")


def run_device(device, fixtures, output, repeats, smoke, selected_versions, selected_cases):
    env = dict(os.environ, HORIZON_BENCH_FIXTURES=str(fixtures),
               HORIZON_BENCH_SESSION=output.name,
               HORIZON_BENCH_MANIFEST=str(output / "apks.json"), PYTHONIOENCODING="utf-8")
    driver = Path(__file__).with_name("android_benchmark_device.py")
    jobs = []
    cases = (("water", 16, 0),) if smoke else selected_cases
    for repeat in range(repeats):
        for index, (scenario, distance, lod) in enumerate(cases):
            versions = selected_versions
            if (repeat + index) % 2:
                versions = tuple(reversed(versions))
            command = [sys.executable, str(driver), device, scenario, "--distance", str(distance),
                       "--lod-distance", str(lod), "--versions", *versions]
            log = output / f"{device}-{repeat}-{index}-{scenario}.log"
            started = time.time()
            with log.open("w", encoding="utf8") as stream:
                result = subprocess.run(command, env=env, stdout=stream, stderr=subprocess.STDOUT)
            jobs.append(dict(scenario=scenario, distance=distance, lod=lod, versions=versions,
                             exit_code=result.returncode, elapsed_seconds=time.time() - started, log=str(log)))
            (output / f"{device}-jobs.json").write_text(json.dumps(jobs, indent=2), encoding="utf8")
            print(device, scenario, "exit", result.returncode, flush=True)
            if result.returncode:
                # A failed launch invalidates automation coordinates and needs inspection.
                break
        else:
            continue
        break


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--devices", nargs="+", choices=("poco", "tablet"), default=["poco", "tablet"])
    parser.add_argument("--repeats", type=int, default=2)
    parser.add_argument("--versions", nargs="+", choices=VERSIONS, default=list(DEFAULT_VERSIONS))
    case_ids = {f"{name}-{distance}-{lod}": (name, distance, lod) for name, distance, lod in CASES}
    parser.add_argument("--cases", nargs="+", choices=case_ids, default=list(case_ids))
    parser.add_argument("--session", default="benchmark-matrix", help="Output directory name inside private fixtures")
    parser.add_argument("--smoke", action="store_true")
    parser.add_argument("--summarize", action="store_true")
    args = parser.parse_args()
    fixtures = args.fixtures.resolve()
    if args.repeats < 1:
        parser.error("--repeats must be positive")
    if Path(args.session).name != args.session or args.session in (".", ".."):
        parser.error("--session must be a directory name")
    output = fixtures / args.session
    output.mkdir(exist_ok=True)
    if not args.summarize:
        names = [f"Horizon-{v}-benchmark.apk" if v in ("0.7.3", "0.7.7") else f"cpu-{v}.apk" for v in args.versions]
        manifest = {name: hashlib.sha256((fixtures / name).read_bytes()).hexdigest() for name in names}
        (output / "apks.json").write_text(json.dumps(manifest, indent=2), encoding="utf8")
        with concurrent.futures.ThreadPoolExecutor(max_workers=len(args.devices)) as pool:
            tasks = [pool.submit(run_device, d, fixtures, output, args.repeats, args.smoke,
                                 tuple(args.versions), [case_ids[c] for c in args.cases]) for d in args.devices]
            for task in tasks:
                task.result()
    summarize(fixtures, output / "results.json")


if __name__ == "__main__":
    main()
