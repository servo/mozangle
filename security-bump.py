#!/usr/bin/env python3

import os
import re
import subprocess
import tempfile
from pathlib import Path

upstream_txt = Path("UPSTREAM").read_text()

tag, commit = upstream_txt.removeprefix("gfx/angle is taken from ").split(": ", 1)

print(f"Existing Firefox tag: {tag} and commit: {commit}")

esr = tag.removeprefix("FIREFOX_").split("_", 1)[0]

print(f"ESR: {esr}")

with tempfile.TemporaryDirectory() as tmpdir:
    subprocess.run(
        [
            "git",
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            "https://github.com/mozilla-firefox/firefox.git",
            str(tmpdir),
        ],
        check=True,
    )

    subprocess.run(
        ["git", "fetch", "--tags"],
        cwd=tmpdir,
        check=True,
    )

    latest_tag = (
        subprocess.check_output(
            [
                "git",
                "for-each-ref",
                "--sort=-creatordate",
                "--format=%(refname:short)",
                f"refs/tags/FIREFOX_{esr}_*_RELEASE",
            ],
            cwd=tmpdir,
        )
        .decode()
        .splitlines()[0]
    )

    latest_commit = (
        subprocess.check_output(["git", "rev-list", "-n", "1", latest_tag], cwd=tmpdir)
        .decode()
        .strip()
    )

print(f"Latest Firefox tag: {latest_tag} and commit: {latest_commit}")

Path("UPSTREAM").write_text(f"gfx/angle is taken from {latest_tag}: {latest_commit}\n")

if GITHUB_OUTPUT := os.getenv("GITHUB_OUTPUT"):
    with open(GITHUB_OUTPUT, "a") as github_output_file:
        print(f"tag={latest_tag}", file=github_output_file)
        print(f"commit={latest_commit}", file=github_output_file)

# bump cargo.toml

toml_path = Path("Cargo.toml")

toml_text = toml_path.read_text()
# extract create version
version_match = re.search(
    r'^\s*version\s*=\s*"(\d+)\.(\d+)\.(\d+)"',
    toml_text,
    re.MULTILINE,
)
version_major, version_minor, version_patch = map(int, version_match.groups())
print(f"Current mozangle version: {version_major}.{version_minor}.{version_patch}")
toml_text = re.sub(
    r'version = "\d+\.\d+\.\d+"\n',
    f'version = "{version_major}.{version_minor}.{version_patch + 1}"\n',
    toml_text,
)
print(
    f"Bumped mozangle version to: {version_major}.{version_minor}.{version_patch + 1}"
)

toml_path.write_text(toml_text)
