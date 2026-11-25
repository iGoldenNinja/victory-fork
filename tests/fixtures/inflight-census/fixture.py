"""Run a CPU-only regression for the in-flight readback census.

From a checkout with Python 3, Git and rustc on PATH, run
    python tests/fixtures/inflight-census/fixture.py [--revision <commit>]
On Windows use a Developer Command Prompt with the native linker available.

The test uses the checkout's landing, span, ticket and census code with
ordinary arrays in place of completed D3D12 buffers. The stub merge consumes
the input but writes zero bytes. Device submission and logging are outside
the scope of this test.
"""

import argparse
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

here = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--repo", type=Path, default=Path.cwd(), help="repository root")
parser.add_argument("--revision", help="use checked-in source at this revision")
args = parser.parse_args()

def source(name):
    path = f"crates/victory-gpu/src/{name}.rs"
    if args.revision:
        return subprocess.check_output(["git", "-C", args.repo, "show", f"{args.revision}:{path}"], encoding="utf-8")
    return (args.repo / path).read_text(encoding="utf-8")

def item(text, name):
    # The original item is embedded verbatim. All sampled items have a closing
    # brace on its own line at declaration indentation.
    pattern = rf"^(?P<indent>[ \t]*)(?:(?:pub\(crate\)|pub) )?(?:fn|struct) {name}\b.*?^(?P=indent)\}}[ \t]*$"
    match = re.search(pattern, text, re.MULTILINE | re.DOTALL)
    if not match:
        raise ValueError(f"Cannot locate Rust item {name}")
    return match.group()

host = source("host")
shadow = source("shadow")
census = source("census")
code = (here / "fixture.template").read_text(encoding="utf-8")
for origin, names in [(host, ["relay_card_inflight", "flagged_pieces", "inflight_gets_slot", "inflight_piece_from"]),
                      (shadow, ["outside", "CardSpan", "piece_at", "card_piece_of"]),
                      (census, ["on", "fetched", "compared", "Copy"])]:
    for name in names:
        code = code.replace(f"@{name}@", item(origin, name))

with tempfile.TemporaryDirectory(prefix="inflight-census-") as tmp:
    path = Path(tmp) / "fixture.rs"
    path.write_text(code, encoding="utf-8")
    exe = Path(tmp) / ("fixture.exe" if os.name == "nt" else "fixture")
    subprocess.run([os.environ.get("RUSTC", "rustc"), "--edition=2024", "--test",
                    "-Adead_code", path, "-o", exe], check=True, cwd=tmp)
    env = dict(os.environ, VICTORY_CENSUS="1")
    sys.exit(subprocess.run([exe, "--test-threads=1", "--nocapture"], env=env).returncode)
