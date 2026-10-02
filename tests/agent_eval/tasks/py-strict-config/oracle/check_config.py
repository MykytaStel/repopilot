"""Hidden check: config.py passes strict mypy with its suppressions removed,
and the documented defaults hold."""

import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

source = Path("config.py").read_text()
stripped = re.sub(r"#\s*(type:\s*ignore|mypy:)[^\n]*", "", source)
with tempfile.TemporaryDirectory() as tmp:
    Path(tmp, "config.py").write_text(stripped)
    Path(tmp, "mypy.ini").write_text("[mypy]\nstrict = True\nfiles = config.py\n")
    run = subprocess.run(["mypy"], cwd=tmp, capture_output=True, text=True)
    if run.returncode != 0:
        sys.exit("strict mypy fails without suppressions:\n" + run.stdout)

sys.path.insert(0, ".")
from config import hosts, load, port  # noqa: E402

assert port({}) == 8080, port({})
assert port({"port": 9000}) == 9000
assert hosts({}) == []
assert hosts({"hosts": ["a.example"]}) == ["a.example"]
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as handle:
    json.dump({"port": 7000}, handle)
assert port(load(handle.name)) == 7000
print("ok")
