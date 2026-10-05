#!/usr/bin/env python3
"""Observed publisher runtimes; unknown tags need recorded release evidence."""
import json
import sys


# Source: successful npm publisher jobs in these public release runs.
# v0.22.0: 33184346391; v0.23.0: 36046172579;
# v0.24.0-rc.1: 37030080572; v0.24.0: 37064025026; v0.24.1: 37114008638;
# v0.24.2: 37281106606.
RUNTIMES = {
    "v0.22.0": {"node": "24.19.0", "npm": "11.17.0"},
    "v0.23.0": {"node": "24.21.0", "npm": "11.19.0"},
    "v0.24.0-rc.1": {"node": "24.21.0", "npm": "11.19.0"},
    "v0.24.0": {"node": "24.21.0", "npm": "11.19.0"},
    "v0.24.1": {"node": "24.21.0", "npm": "11.19.0"},
    "v0.24.2": {"node": "24.21.0", "npm": "11.19.0"},
}


def main() -> int:
    tag = sys.argv[1] if len(sys.argv) == 2 else ""
    runtime = RUNTIMES.get(tag)
    if runtime is None:
        print(
            f"unsupported publication tag: {tag or 'missing'}; record its "
            "publisher Node/npm runtime in scripts/publication_runtime.py "
            "before verification, or use the historical tag's verifier",
            file=sys.stderr,
        )
        return 1
    print(json.dumps(runtime))
    return 0


if __name__ == "__main__":
    sys.exit(main())
