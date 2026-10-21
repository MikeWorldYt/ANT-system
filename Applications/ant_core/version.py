# App versions come from each app's version.txt, the file PyInstaller stamps into the exe.
import os
import re

_FILE_VERSION = re.compile(r"StringStruct\('FileVersion',\s*'([\d.]+)'\)")


def parse_file_version(version_txt: str) -> str | None:
    m = _FILE_VERSION.search(version_txt)
    return f"v{m.group(1)}" if m else None


def read_app_version(app_dir: str) -> str:
    try:
        with open(os.path.join(app_dir, "version.txt"), encoding="utf-8") as f:
            return parse_file_version(f.read()) or "dev"
    except OSError:
        return "dev"


def parse_version(text: str) -> tuple[int, ...]:
    return tuple(int(n) for n in re.findall(r"\d+", text)[:3])
