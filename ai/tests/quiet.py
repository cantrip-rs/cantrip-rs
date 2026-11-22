"""What every test of the training driver wants of this machine, set once on import: no notification leaves a test
(`notify.py` reads the environment and a per-user file), a generation's engine is not read from this repository
(`engine.identify` asks git of 50,000 files: seconds a call) and nothing is pinned, and the preflight's room on the disk
and in memory does not depend on the machine the tests run on. A test of one of these patches it back."""

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import engine, generation, notify  # noqa: E402

os.environ[notify.ENV_FILE] = os.devnull
os.environ.pop(notify.ENV_HOOK, None)
os.environ.pop(notify.ENV_WEBHOOK, None)

UNPINNED = dict(id=None, commit=None, dirty=None)
engine.identify = lambda repo: dict(UNPINNED)
generation.available_memory = lambda: 64 * 2**30
generation.disk_free = lambda path: 2**40
