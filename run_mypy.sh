#!/bin/bash
cd /home/stan/Prod/maestro
mypy maestro/memory/migrations/memori_migration.py \
    maestro/memory/tests/test_performance.py \
    maestro/memory/tests/test_concurrency.py \
    maestro/memory/tests/test_agent_types.py \
    maestro/memory/tests/unit/test_models.py \
    maestro/memory/tests/unit/test_migrations.py \
    maestro/memory/tests/unit/test_managers.py \
    maestro/memory/tests/e2e/test_migration_e2e.py \
    maestro/memory/tests/e2e/test_maestro_complete_workflow.py \
    maestro/memory/tests/unit/test_service_edge_cases.py \
    maestro/memory/dashboard.py \
    maestro/memory/tests/test_dashboard_serving.py \
    --ignore-missing-imports \
    --show-error-codes