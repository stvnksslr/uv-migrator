"""UV Migrator - Tool for converting Python projects to use uv."""

import sys
from typing import List

try:
    from ._uv_migrator import run_cli
    _has_rust_extension = True
except ImportError:
    _has_rust_extension = False
    
    def run_cli(args: List[str]) -> None:
        """Fallback implementation when Rust extension is not available."""
        import shutil
        import subprocess
        
        binary = shutil.which("uv-migrator")
        if binary:
            subprocess.run([binary] + args, check=True)
        else:
            raise RuntimeError(
                "uv-migrator binary not found. "
                "Please install the binary version with: cargo install uv-migrator"
            )

__version__ = "2025.8.0"

def main() -> None:
    """Main entry point for the CLI."""
    try:
        run_cli(sys.argv[1:])
    except KeyboardInterrupt:
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

def has_rust_extension() -> bool:
    """Check if the Rust extension is available."""
    return _has_rust_extension

if __name__ == "__main__":
    main()