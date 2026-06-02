"""Make the harness package importable and locate the fixtures dir.

Tests run with plain ``unittest`` (no install step), so each test imports this to
put ``harness/`` on ``sys.path`` and resolve fixture paths.
"""
import sys
from pathlib import Path

HARNESS = Path(__file__).resolve().parents[1]
FIXTURES = HARNESS / "fixtures"
if str(HARNESS) not in sys.path:
    sys.path.insert(0, str(HARNESS))
