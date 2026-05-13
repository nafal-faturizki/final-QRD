"""Python-side QRD SDK scaffold.

This module defines the public surface that will later be backed by PyO3.
"""

from __future__ import annotations


class FileReader:
    """Placeholder QRD file reader.

    The future implementation will delegate to Rust core bindings.
    """

    def __init__(self, path: str, master_key: bytes | None = None) -> None:
        self.path = path
        self.master_key = master_key

    def inspect_header(self) -> dict[str, object]:
        raise NotImplementedError("PyO3 binding not added yet")

    def inspect_footer(self) -> dict[str, object]:
        raise NotImplementedError("PyO3 binding not added yet")


class FileWriter:
    """Placeholder QRD file writer."""

    def __init__(self, path: str, schema: dict[str, object]) -> None:
        self.path = path
        self.schema = schema

    def write_row(self, row: dict[str, object]) -> None:
        raise NotImplementedError("PyO3 binding not added yet")

    def finish(self) -> None:
        raise NotImplementedError("PyO3 binding not added yet")


def inspect_header(path: str) -> dict[str, object]:
    """Inspect a QRD header.

    Example:
        >>> inspect_header("example.qrd")
        Traceback (most recent call last):
        ... NotImplementedError: PyO3 binding not added yet
    """

    raise NotImplementedError("PyO3 binding not added yet")


def inspect_footer(path: str) -> dict[str, object]:
    """Inspect a QRD footer."""

    raise NotImplementedError("PyO3 binding not added yet")
