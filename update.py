#!/usr/bin/env python3

# update gfx/angle based on the tag specified in UPSTREAM

import tarfile
import urllib.request
from pathlib import Path

upstream_txt = Path("UPSTREAM").read_text()

tag = upstream_txt.removeprefix("gfx/angle is taken from ").split(":", 1)[0].strip()

print(f"Using Firefox tag: {tag}")

url = f"https://github.com/mozilla-firefox/firefox/archive/refs/tags/{tag}.tar.gz"
prefix = f"firefox-{tag}/gfx/angle/"

print(f"Downloading {url}")

with urllib.request.urlopen(url) as response:
    print("Extracting")

    with tarfile.open(fileobj=response, mode="r|gz") as archive:
        for member in archive:
            if not member.name.startswith(prefix):
                continue

            member.name = member.name[len(prefix) :]

            if member.name:
                archive.extract(member, path="gfx/angle")


for patch in Path("patches").glob("*.patch"):
    print(f"Applying patch: {patch}")
    import subprocess

    subprocess.run(["git", "apply", str(patch), "--reject"], check=True)

from generate_build_data import run

run()
