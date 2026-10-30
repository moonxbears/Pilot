import os
import sys
import json
import argparse
from pathlib import Path

def install_kernel_spec(user=True, prefix=None):
    if prefix:
        dest_dir = Path(prefix) / "share" / "jupyter" / "kernels" / "pilot"
    elif user:
        dest_dir = Path.home() / ".local" / "share" / "jupyter" / "kernels" / "pilot"
    else:
        dest_dir = Path("/usr/local/share/jupyter/kernels/pilot")

    dest_dir.mkdir(parents=True, exist_ok=True)

    kernel_json = {
        "argv": [
            sys.executable,
            "-m",
            "pilot_kernel",
            "-f",
            "{connection_file}"
        ],
        "display_name": "Pilot",
        "language": "pilot",
        "interrupt_mode": "signal",
        "metadata": {
            "debugger": False,
            "language_info": {
                "name": "pilot",
                "codemirror_mode": "python",
                "file_extension": ".pilot",
                "mimetype": "text/x-pilot"
            }
        }
    }

    json_path = dest_dir / "kernel.json"
    with open(json_path, "w", encoding="utf-8") as f:
        json.dump(kernel_json, f, indent=2)

    print(f"Successfully installed Pilot Jupyter kernel spec to: {dest_dir}")
    print("Pilot is now available in Jupyter Notebook, JupyterLab, and VS Code Notebooks!")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Install Pilot Jupyter Kernel Spec")
    parser.add_argument("--user", action="store_true", default=True, help="Install for current user (default)")
    parser.add_argument("--sys-prefix", action="store_true", help="Install into sys.prefix")
    parser.add_argument("--prefix", type=str, help="Install into specific prefix")

    args = parser.parse_args()
    target_prefix = sys.prefix if args.sys_prefix else args.prefix
    install_kernel_spec(user=not args.sys_prefix and not args.prefix, prefix=target_prefix)
