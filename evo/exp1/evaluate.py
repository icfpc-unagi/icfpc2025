import argparse
import json
import os
import re
import shutil
import subprocess
import time
from pathlib import Path
from typing import Any


TIMEOUT_SEC = 60
MAX_TEXT_LEN = 10000  # cap stdout/stderr saved into metrics
TARGET_VALUE = 4_293_874
DEFAULT_STDIN = "local random 18 0\n"


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


def main(program_path: str, results_dir: str) -> None:
    print(f"Preparing to run via cargo: {program_path}")
    print(f"Results directory: {results_dir}")

    # Base dir: ../../ from this file (repo root with Cargo.toml)
    here = Path(__file__).resolve()
    # ../../ relative to this file's directory => repo root (icfpc2025)
    base_dir = here.parents[2]
    src_bin_dir = base_dir / "src" / "bin"
    tmp_rs_path = src_bin_dir / "tmp.rs"
    shutil.copyfile(program_path, tmp_rs_path)

    # Run with cargo (quiet so stdout is only program output)
    cmd = ["cargo", "run", "--release", "--bin", "tmp"]
    print(f"Running: {' '.join(cmd)} (cwd={base_dir})")

    exec_start = time.perf_counter()
    try:
        run_proc = subprocess.run(
            cmd,
            cwd=str(base_dir),
            capture_output=True,
            text=True,
            timeout=TIMEOUT_SEC,
            input=DEFAULT_STDIN,
        )
        exec_end = time.perf_counter()
        exec_time = exec_end - exec_start

        process_ok = run_proc.returncode == 0
        stdout_raw = run_proc.stdout or ""
        stdout_stripped = stdout_raw.strip()
        stdout_int_val = None
        stdout_is_int = False

        print("=== stdout ===")
        print(_trim(run_proc.stdout))
        print("=== stderr ===")
        print(_trim(run_proc.stderr))

        # Parse special logs from stderr
        status_value: str | None = None
        score_value: int | None = None
        if run_proc.stderr:
            status_re = re.compile(r"^\s*!log\s+status\b:?\s*(\S.*)$")
            score_re = re.compile(r"^\s*!log\s+score\b:?\s*(-?\d+)")
            lines = run_proc.stderr.splitlines()
            # Pass 1: determine final status
            for line in lines:
                m = status_re.match(line)
                if m:
                    status_value = m.group(1).strip()
            # Pass 2: if AC, extract score (use last occurrence)
            if status_value == "AC":
                for line in lines:
                    m2 = score_re.match(line)
                    if m2:
                        try:
                            score_value = int(m2.group(1))
                        except Exception:
                            score_value = None

        try:
            stdout_int_val = int(stdout_stripped)
            stdout_is_int = True
        except Exception:
            stdout_is_int = False

        success = process_ok and stdout_is_int
        if not process_ok:
            err = f"non-zero exit: {run_proc.returncode}"
        elif not stdout_is_int:
            err = "stdout is not an integer"
        else:
            err = None

        combined_score = float(abs((stdout_int_val or 0) - TARGET_VALUE)) if success else 0.0

        metrics = {
            "combined_score": -combined_score,
            "execution_time_sec": exec_time,
            "return_code": run_proc.returncode,
        }
        # Attach parsed status and conditional score
        if status_value is not None:
            metrics["status"] = status_value
            if status_value == "AC" and score_value is not None:
                metrics["score"] = score_value
        save_results(results_dir, correct=success, metrics=metrics, error=err)
    except subprocess.TimeoutExpired as e:
        exec_end = time.perf_counter()
        exec_time = exec_end - exec_start
        print(f"Execution timed out after {TIMEOUT_SEC} seconds.")
        err = f"timeout after {TIMEOUT_SEC}s"
        metrics = {
            "combined_score": 0.0,
            "execution_time_sec": exec_time,
            "timed_out": True,
        }
        save_results(results_dir, correct=False, metrics=metrics, error=err)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Evaluate a Rust single-file program")
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
