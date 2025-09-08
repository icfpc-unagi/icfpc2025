import subprocess
from pathlib import Path
d = {
    f"{p.parent.name}_{p.stem}": subprocess.run(
        ["./target/release/reduce_graph"],
        check=True,
        input=p.read_text(),
        text=True,
        capture_output=True,
    ).stdout
    for p in sorted(Path("./localtest/in").glob("*/*.json"))
}
print('export const data = {')
for k, v in d.items():
    print(f"  {k}: {v.strip()},")
print('};')
