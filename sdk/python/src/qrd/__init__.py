"""QRD Python SDK scaffold.

The real implementation will bind to the Rust core via PyO3.
"""

from .core import FileReader, FileWriter, inspect_footer, inspect_header

__all__ = ["FileReader", "FileWriter", "inspect_footer", "inspect_header"]
__version__ = "0.1.0"
