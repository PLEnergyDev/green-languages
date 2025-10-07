from datetime import datetime, timezone
from typing import Any
from glob import glob
import subprocess
import os


class ProgramError(Exception):
    def __init__(
        self, failed: str | None = None, ex: Exception | None = None, *args
    ) -> None:
        self.failed = failed
        self.ex = ex
        super().__init__(*args)

    def __str__(self) -> str:
        err_msg = ""
        if self.failed:
            err_msg += f"{self.failed}"
        if self.ex:
            if self.failed:
                err_msg += f" - {self.ex}"
            else:
                err_msg += str(self.ex)
        if not err_msg:
            return "failed."
        return f"{err_msg}."


def write_file(data: str | bytes, path: str) -> None:
    try:
        with open(path, "wb") as file:
            if isinstance(data, str):
                file.write(data.encode())
            else:
                file.write(data)
    except OSError as ex:
        raise ProgramError("failed while writing file", ex)


def write_file_sudo(data: str | bytes, path: str) -> None:
    if isinstance(data, str):
        data = data.encode()
    try:
        subprocess.run(
            ["sudo", "tee", path], input=data, check=True, stdout=subprocess.DEVNULL
        )
    except subprocess.CalledProcessError as ex:
        raise ProgramError("failed while writing file with superuser priviledges", ex)


def read_file(path: str) -> str:
    if not os.path.exists(path):
        raise ProgramError(f"file {path} doesn't exist")

    try:
        with open(path, "r") as file:
            return file.read().strip()
    except OSError as ex:
        raise ProgramError("failed while reading file", ex)
