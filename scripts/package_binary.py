#!/usr/bin/env python3
"""Package an already built dip binary with the redistribution notices."""

import argparse
from pathlib import Path
import tarfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    notices = [
        root / "LICENSE",
        root / "THIRD_PARTY_NOTICES.md",
        root / "assets/drawio/README.md",
        root / "assets/drawio/licenses",
        root / "assets/licenses",
    ]
    # Require the generated inventory even if a checkout accidentally omits it.
    for path in [args.binary, root / "assets/licenses/cargo-dependencies.txt"]:
        if not path.is_file():
            parser.error(f"missing distribution input: {path}")
    for path in notices:
        if not path.exists():
            parser.error(f"missing distribution input: {path}")
    if args.archive.resolve() == args.binary.resolve() or any(
        args.archive.resolve() == path.resolve()
        or (path.is_dir() and args.archive.resolve().is_relative_to(path.resolve()))
        for path in notices
    ):
        parser.error("archive must not overwrite a distribution input")
    # Exclusive creation avoids replacing an existing release archive.
    with tarfile.open(args.archive, "x:gz") as archive:
        archive.add(args.binary, arcname=args.binary.name)
        for path in notices:
            archive.add(path, arcname=path.relative_to(root))
    print(f"Packaged {args.archive} with license notices")


if __name__ == "__main__":
    main()
