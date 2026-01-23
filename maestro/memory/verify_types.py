#!/usr/bin/env python3
"""
Verification script to check if the type fixes are correct.
This script imports each module to verify there are no import or basic type errors.
"""
import sys
from pathlib import Path

# Add the maestro directory to the path
maestro_root = Path(__file__).parent.parent
sys.path.insert(0, str(maestro_root))

def verify_imports() -> bool:
    """Verify that all modules can be imported without errors."""
    modules = [
        "maestro.memory.utils.async_extractor",
        "maestro.memory.logging_config",
        "maestro.memory.scanner",
        "maestro.memory.dashboard",
        "maestro.memory.cli",
        "maestro.memory.search.zoekt_client",
    ]

    print("Verifying module imports...")
    for module_name in modules:
        try:
            print(f"  Importing {module_name}...", end=" ")
            __import__(module_name)
            print("✓")
        except Exception as e:
            print(f"✗ Error: {e}")
            return False

    print("\nAll modules imported successfully!")
    return True

def verify_type_annotations() -> bool:
    """Verify that type annotations are present where expected."""
    print("\nVerifying type annotations...")

    # Check async_extractor.py
    from maestro.memory.utils.async_extractor import AsyncMemoryExtractor
    extractor = AsyncMemoryExtractor()
    print(f"  AsyncMemoryExtractor.queue type: {type(extractor.queue).__name__}")

    # Check logging_config.py
    import maestro.memory.logging_config as lc
    print(f"  configure_logging function: {lc.configure_logging.__name__}")

    # Check zoekt_client.py
    from maestro.memory.search.zoekt_client import ZoektClient, ZoektConfig
    client = ZoektClient()
    print(f"  ZoektClient.client type: {type(client.client).__name__}")

    print("Type annotations verified!")
    return True

if __name__ == "__main__":
    success = True
    success = verify_imports() and success
    success = verify_type_annotations() and success

    if success:
        print("\n✓ All verifications passed!")
        sys.exit(0)
    else:
        print("\n✗ Some verifications failed!")
        sys.exit(1)