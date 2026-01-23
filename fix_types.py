#!/usr/bin/env python3
"""Script to fix mypy type annotations in test files."""
import re
from pathlib import Path

def fix_test_file(file_path: Path) -> None:
    """Fix type annotations in a single test file."""
    content = file_path.read_text()

    # Fix 1: Change test function return types from -> Any to -> None
    # Pattern: async def test_xxx(...) -> Any:
    content = re.sub(
        r'(\s+async def test_\w+\([^)]*\)) -> Any:\s*\n\s*"""',
        r'\1 -> None:\n        """',
        content
    )

    # Pattern: def test_xxx(...) -> Any:
    content = re.sub(
        r'(\s+def test_\w+\([^)]*\)) -> Any:\s*\n\s*"""',
        r'\1 -> None:\n        """',
        content
    )

    # Fix 2: Change fixture return types
    # Pattern: @pytest.fixture\ndef xxx() -> Any:
    content = re.sub(
        r'(@pytest\.fixture\s*\ndef \w+\([^)]*\)) -> Any:\s*\n\s*"""',
        r'\1 -> Generator[None, None, None]:\n        """',
        content
    )

    # Pattern: @pytest_asyncio.fixture\nasync def xxx() -> AsyncGenerator:
    content = re.sub(
        r'(@pytest_asyncio\.fixture\s*\n\s*async def \w+\([^)]*\)) -> AsyncGenerator:\s*\n\s*"""',
        r'\1 -> AsyncGenerator[None, None]:\n        """',
        content
    )

    # Fix 3: Change dict to dict[str, Any] for fixtures
    # Pattern: def xxx() -> dict:
    content = re.sub(
        r'(@pytest\.fixture\s*\ndef \w+\([^)]*\)) -> dict:\s*\n\s*"""',
        r'\1 -> dict[str, Any]:\n        """',
        content
    )

    # Fix 4: Change list to list[str] for fixtures
    # Pattern: def xxx() -> list:
    content = re.sub(
        r'(@pytest\.fixture\s*\ndef \w+\([^)]*\)) -> list:\s*\n\s*"""',
        r'\1 -> list[str]:\n        """',
        content
    )

    # Fix 5: Change Any to None for other methods
    # Pattern: def xxx(...) -> Any: (but not test methods or fixtures)
    lines = content.split('\n')
    new_lines = []
    i = 0
    while i < len(lines):
        line = lines[i]
        # Check if it's a method definition with -> Any
        if re.search(r'\s+def \w+\([^)]*\) -> Any:\s*$', line):
            # Check if it's NOT a test method or fixture
            if not re.search(r'def test_\w+', line) and not re.search(r'@pytest', lines[i-1] if i > 0 else ''):
                # Change -> Any to -> None
                line = re.sub(r' -> Any:\s*$', ' -> None:', line)
        new_lines.append(line)
        i += 1

    content = '\n'.join(new_lines)

    # Write back
    file_path.write_text(content)
    print(f"Fixed: {file_path}")

def main():
    """Fix all test files."""
    test_dir = Path("maestro/memory/tests")

    # Files to fix
    files_to_fix = [
        "test_agent_types.py",
        "test_concurrency.py",
        "test_dashboard_serving.py",
        "test_dashboard_with_sample_data.py",
        "test_llm_enhancement.py",
        "test_migration.py",
        "test_performance.py",
        "test_security.py",
        "api/test_routes.py",
        "cli/test_cli_commands.py",
    ]

    for file_name in files_to_fix:
        file_path = test_dir / file_name
        if file_path.exists():
            fix_test_file(file_path)
        else:
            print(f"Not found: {file_path}")

if __name__ == "__main__":
    main()