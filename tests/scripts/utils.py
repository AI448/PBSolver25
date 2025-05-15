from typing import Iterable
from pathlib import Path
import os


def enumerate_files(path: str | Path) -> Iterable[Path]:
    if not isinstance(path, Path):
        path = Path(path)

    if os.path.isfile(path):
        yield path
    else:
        for child in os.listdir(path):
            yield from enumerate_files(os.path.join(path, child))
