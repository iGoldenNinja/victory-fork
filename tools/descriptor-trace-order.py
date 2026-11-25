"""Print captured texture operations in D3D12 submission order from a local diagnostics log."""
import argparse
import re
from pathlib import Path


FIELD = re.compile(r'(\w+)=("[^"]*"|[^ ]+)')


def timeline(lines):
    recorded = {}
    submissions = []
    for line in lines:
        if "descriptor trace " not in line:
            continue
        event, _, rest = line.split("descriptor trace ", 1)[1].strip().partition(" ")
        fields = {key: value.strip('"') for key, value in FIELD.findall(rest)}
        if "id" not in fields or "frame" not in fields:
            continue
        ident = int(fields["id"])
        if event in ("marker", "binding"):
            recorded[ident] = fields
        elif event == "submit":
            # The renderer holds the host submission lock here. Log order breaks
            # ties for multiple observations in the same physical command list.
            submissions.append((
                int(fields["fence"]), int(fields["position"]), len(submissions),
                ident, int(fields["native"]),
            ))
    resources = {}
    writes = {}
    rows = []
    for fence, position, _, ident, native in sorted(submissions):
        entry = recorded.get(ident)
        if entry is None:
            continue
        role = entry["role"]
        bound = int(entry["native"])
        # Copy markers retain the resource itself. Bindings retain a heap slot
        # whose contents may change between recording and submission.
        actual = native if role in ("compute-input", "reader", "writer") else bound
        texture = resources.setdefault(actual, len(resources) + 1) if actual else None
        last_write = writes.get(texture) if role in ("reader", "target-preview") else None
        stable = bound == actual if role in ("compute-input", "reader", "writer") else None
        rows.append((int(entry["frame"]), fence, position, ident, role, texture, stable, last_write, entry.get("shader")))
        if role == "writer" and texture is not None:
            writes[texture] = ident
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("--frame", type=int, help="include this CPU recording frame and each adjacent frame")
    args = parser.parse_args()
    with args.log.open(encoding="utf-8", errors="replace") as stream:
        rows = timeline(stream)
    if args.frame is not None:
        rows = [row for row in rows if abs(row[0] - args.frame) <= 1]
    if not rows:
        parser.exit(message="No submitted descriptor trace events in that interval.\n")
    base = rows[0][1]
    print("recording_frame\tfence_delta\tlist_index\tevent\trole\ttexture\tsame_slot_resource\tlast_captured_write\tshader")
    for frame, fence, position, ident, role, texture, stable, last, shader in rows:
        print(frame, fence - base, position, ident, role, texture or "-", "-" if stable is None else str(stable),
              last or "-", shader or "-", sep="\t")


if __name__ == "__main__":
    main()
