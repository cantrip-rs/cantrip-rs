"""`cantrip-workers`, run from the repository (tools/cantrip-workers.cmd, tools/cantrip-workers): docs/workers.md."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from cantrip_workers.cli import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main())
