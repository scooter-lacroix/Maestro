#!/usr/bin/env python3
"""
Run mypy on specific files and capture output
"""
import subprocess
import sys

files = [
    "maestro/memory/migrations/memori_migration.py",
    "maestro/memory/tests/test_performance.py",
    "maestro/memory/tests/test_concurrency.py",
    "maestro/memory/tests/test_agent_types.py",
    "maestro/memory/tests/unit/test_models.py",
    "maestro/memory/tests/unit/test_migrations.py",
    "maestro/memory/tests/unit/test_managers.py",
    "maestro/memory/tests/e2e/test_migration_e2e.py",
    "maestro/memory/tests/e2e/test_maestro_complete_workflow.py",
    "maestro/memory/tests/unit/test_service_edge_cases.py",
    "maestro/memory/dashboard.py",
    "maestro/memory/tests/test_dashboard_serving.py",
]

cmd = [
    sys.executable, "-m", "mypy",
    *files,
    "--ignore-missing-imports",
    "--show-error-codes",
]

print("Running mypy...")
print("Command:", " ".join(cmd))
print("\n" + "="*80 + "\n")

result = subprocess.run(cmd, cwd="/home/stan/Prod/maestro", capture_output=True, text=True)

print(result.stdout)
if result.stderr:
    print("STDERR:", result.stderr)

print("\n" + "="*80)
print(f"Return code: {result.returncode}")

sys.exit(result.returncode)