import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from statistics import median
from typing import Any


TIMEOUT_SEC = 30
MAX_TEXT_LEN = 10000  # cap stdout/stderr saved into metrics
N_TESTS = 63 * 3
N_WORKERS = 63
N_ROOMS = 18


def _trim(text: str, limit: int = MAX_TEXT_LEN) -> str:
    if text is None:
        return ""
    if len(text) <= limit:
        return text
    head = text[: limit - 200]
    tail = text[-200:]
    return f"{head}\n...\n[truncated {len(text) - limit} chars]\n...\n{tail}"


def save_results(results_dir: str, correct: bool, metrics: dict, error: str | None) -> None:
    os.makedirs(results_dir, exist_ok=True)
    correct_payload = {"correct": correct, "error": error}

    # Echo to stdout
    print("=== correct.json ===")
    print(json.dumps(correct_payload, indent=4, ensure_ascii=False))
    print("=== metrics.json ===")
    print(json.dumps(metrics, indent=4, ensure_ascii=False))

    # Persist to files
    with open(os.path.join(results_dir, "correct.json"), "w") as f:
        json.dump(correct_payload, f, indent=4, ensure_ascii=False)
    with open(os.path.join(results_dir, "metrics.json"), "w") as f:
        json.dump(metrics, f, indent=4, ensure_ascii=False)
    print(f"Saved correct.json and metrics.json under: {results_dir}")


def _resolve_target_dir(base_dir: Path) -> Path:
    tdir = os.environ.get("CARGO_TARGET_DIR")
    if tdir:
        p = Path(tdir)
        return p if p.is_absolute() else (base_dir / p)
    return base_dir / "target"


def _build_unique_bin(program_path: str, results_dir: str, base_dir: Path) -> tuple[Path, float, str | None, str | None]:
    """
    Create a unique bin under src/bin, build it once, then remove the .rs immediately.
    Returns (binary_path, compile_time_sec, compile_stdout, compile_stderr) where stdout/stderr are trimmed.
    Raises RuntimeError if build fails.
    """
    # Unique name per job using results_dir hash + pid
    results_abs = str(Path(results_dir).resolve())
    hash8 = hashlib.sha1(results_abs.encode("utf-8")).hexdigest()[:8]
    bin_name = f"eval_{hash8}_{os.getpid()}"

    src_bin_dir = base_dir / "src" / "bin"
    src_bin_dir.mkdir(parents=True, exist_ok=True)
    bin_rs_path = src_bin_dir / f"{bin_name}.rs"

    # Copy user program into unique bin
    shutil.copyfile(program_path, bin_rs_path)

    cmd = ["cargo", "build", "--release", "--bin", bin_name]
    print(f"Building: {' '.join(cmd)} (cwd={base_dir})")
    t0 = time.perf_counter()
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(base_dir),
            capture_output=True,
            text=True,
        )
    finally:
        # Delete the .rs as soon as build finishes (success or fail)
        try:
            if bin_rs_path.exists():
                bin_rs_path.unlink()
        except Exception:
            pass
    t1 = time.perf_counter()
    compile_time = t1 - t0

    cout = _trim(proc.stdout or "")
    cerr = _trim(proc.stderr or "")

    if proc.returncode != 0:
        raise RuntimeError(json.dumps({
            "message": "cargo build failed",
            "stdout": cout,
            "stderr": cerr,
        }))

    target_dir = _resolve_target_dir(base_dir)
    bin_path = target_dir / "release" / bin_name
    if sys.platform.startswith("win"):
        bin_path = bin_path.with_suffix(".exe")
    return bin_path, compile_time, cout, cerr


def _parse_status_and_score(stderr_text: str) -> tuple[str | None, int | None]:
    status_value: str | None = None
    score_value: int | None = None
    if stderr_text:
        status_re = re.compile(r"^\s*!log\s+status\b:?\s*(\S.*)$")
        score_re = re.compile(r"^\s*!log\s+score\b:?\s*(-?\d+)")
        lines = stderr_text.splitlines()
        for line in lines:
            m = status_re.match(line)
            if m:
                status_value = m.group(1).strip()
        if status_value == "AC":
            for line in lines:
                m2 = score_re.match(line)
                if m2:
                    try:
                        score_value = int(m2.group(1))
                    except Exception:
                        score_value = None
    return status_value, score_value


def main(program_path: str, results_dir: str) -> None:
    print(f"Preparing to build and evaluate: {program_path}")
    print(f"Results directory: {results_dir}")

    here = Path(__file__).resolve()
    base_dir = here.parents[2]  # repo root (icfpc2025)

    # Build once with a unique bin name
    try:
        bin_path, compile_time, build_stdout, build_stderr = _build_unique_bin(
            program_path, results_dir, base_dir
        )
    except RuntimeError as e:
        # Build failed: record minimal metrics and exit
        msg = str(e)
        metrics = {
            "combined_score": 0.0,
            "compile_time_sec": 0.0,
            "timeout_sec": TIMEOUT_SEC,
            "n_tests": N_TESTS,
            "n_workers": N_WORKERS,
            "build_error": _trim(msg),
        }
        save_results(results_dir, correct=False, metrics=metrics, error="build failed")
        return

    # Helper to run one test case i
    def run_one(i: int) -> dict[str, Any]:
        stdin_data = f"local random {N_ROOMS} {i}\n"
        start = time.perf_counter()
        try:
            p = subprocess.run(
                [str(bin_path)],
                cwd=str(base_dir),
                capture_output=True,
                text=True,
                timeout=TIMEOUT_SEC,
                input=stdin_data,
            )
            end = time.perf_counter()
            exec_time = end - start
            status, score = _parse_status_and_score(p.stderr or "")
            return {
                "return_code": p.returncode,
                "exec_time": exec_time,
                "status": status,
                "score": score,
                "timed_out": False,
                "stdout": p.stdout or "",
                "stderr": p.stderr or "",
            }
        except subprocess.TimeoutExpired:
            end = time.perf_counter()
            exec_time = end - start
            return {
                "return_code": None,
                "exec_time": exec_time,
                "status": None,
                "score": None,
                "timed_out": True,
                "stdout": "",
                "stderr": "",
            }
        except Exception as ex:
            end = time.perf_counter()
            exec_time = end - start
            return {
                "return_code": None,
                "exec_time": exec_time,
                "status": None,
                "score": None,
                "timed_out": False,
                "error": str(ex),
                "stdout": "",
                "stderr": "",
            }

    # Parallel execution of N_TESTS
    results: list[dict[str, Any]] = [None] * N_TESTS  # type: ignore
    with ThreadPoolExecutor(max_workers=N_WORKERS) as ex:
        future_to_idx = {ex.submit(run_one, i): i for i in range(N_TESTS)}
        for fut in as_completed(future_to_idx):
            idx = future_to_idx[fut]
            try:
                results[idx] = fut.result()
            except Exception as ex:
                results[idx] = {
                    "return_code": None,
                    "exec_time": TIMEOUT_SEC,
                    "status": None,
                    "score": None,
                    "timed_out": False,
                    "error": str(ex),
                }

    # Aggregate metrics
    values: list[float] = []
    ac_count = 0
    score2_count = 0
    score2_exec_times: list[float] = []
    for r in results:
        status = r.get("status")
        score = r.get("score")
        exec_time = float(r.get("exec_time", TIMEOUT_SEC))
        if status == "AC":
            ac_count += 1
        is_score2 = (status == "AC" and score == 2)
        if is_score2:
            values.append(exec_time)
            score2_count += 1
            score2_exec_times.append(exec_time)
        else:
            values.append(float(TIMEOUT_SEC))

    # Debug print: build and per-test stdout/stderr (trimmed)
    print("=== build stdout ===")
    print(_trim(build_stdout or ""))
    print("=== build stderr ===")
    print(_trim(build_stderr or ""))
    for idx, r in enumerate(results):
        print(f"=== test {idx} stdout ===")
        print(_trim(r.get("stdout", "")))
        print(f"=== test {idx} stderr ===")
        print(_trim(r.get("stderr", "")))

    # percentile helper (nearest-rank on [0,1])
    def pctl(vals: list[float], p: float) -> float:
        if not vals:
            return float("nan")
        s = sorted(vals)
        k = max(0, min(len(s) - 1, int((len(s) - 1) * p)))
        return float(s[k])

    # Compute summary stats
    min_val = float(min(values)) if values else float("nan")
    p10_val = pctl(values, 0.10)
    p25_val = pctl(values, 0.25)
    p50_val = pctl(values, 0.50)
    p75_val = pctl(values, 0.75)
    p90_val = pctl(values, 0.90)
    max_val = float(max(values)) if values else float("nan")

    # combined_score = negative 25th percentile
    metrics: dict[str, Any] = {
        "combined_score": -p25_val,
        "value_min_sec": min_val,
        "value_p10_sec": p10_val,
        "value_p25_sec": p25_val,
        "value_median_sec": p50_val,
        "value_p50_sec": p50_val,
        "value_p75_sec": p75_val,
        "value_p90_sec": p90_val,
        "value_max_sec": max_val,
        "timeout_sec": TIMEOUT_SEC,
        "n_tests": N_TESTS,
        "n_workers": N_WORKERS,
        "ac_count": ac_count,
        "score2_count": score2_count,
        "score2_rate": (score2_count / N_TESTS),
        "compile_time_sec": compile_time,
        "binary_path": str(bin_path),
    }
    if score2_exec_times:
        metrics["score2_exec_median_sec"] = float(median(score2_exec_times))
        metrics["score2_exec_min_sec"] = float(min(score2_exec_times))

    # Define correctness: at least one test reports status AC
    any_ac = (ac_count > 0)
    save_results(results_dir, correct=any_ac, metrics=metrics, error=None)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Evaluate a Rust single-file program (parallel multi-run)")
    parser.add_argument(
        "--program_path",
        type=str,
        default="initial.rs",
        help="Path to the Rust source file (single file)",
    )
    parser.add_argument(
        "--results_dir",
        type=str,
        default="results",
        help="Directory to save results (metrics.json, correct.json)",
    )
    args = parser.parse_args()
    main(args.program_path, args.results_dir)
