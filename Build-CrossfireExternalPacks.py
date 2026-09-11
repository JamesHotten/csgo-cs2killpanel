"""Compatibility entry point for the organized external Crossfire pack builder."""
from pathlib import Path
import runpy


runpy.run_path(
    str(Path(__file__).resolve().parent / "Tools" / "Crossfire" / "Build-CrossfireExternalPacks.py"),
    run_name="__main__",
)
