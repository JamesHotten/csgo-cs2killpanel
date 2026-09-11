"""Compatibility entry point for the organized Crossfire pack builder."""
from pathlib import Path
import runpy


runpy.run_path(
    str(Path(__file__).resolve().parent / "Tools" / "Crossfire" / "Build-CrossfirePacks.py"),
    run_name="__main__",
)
