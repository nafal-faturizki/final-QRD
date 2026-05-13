import pytest

from qrd import FileReader, FileWriter, inspect_footer, inspect_header


def test_python_sdk_surface_exists() -> None:
    reader = FileReader("example.qrd")
    writer = FileWriter("output.qrd", {"fields": []})

    assert reader.path == "example.qrd"
    assert writer.path == "output.qrd"


def test_python_sdk_placeholders_raise() -> None:
    with pytest.raises(NotImplementedError):
        inspect_header("example.qrd")

    with pytest.raises(NotImplementedError):
        inspect_footer("example.qrd")
