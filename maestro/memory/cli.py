"""
Maestro Memory CLI

Command-line interface for Maestro memory operations.
"""

import sys
import argparse
from pathlib import Path
from loguru import logger

import uvicorn


def serve_command(args: argparse.Namespace) -> int:
    """
    Start the Maestro Memory Dashboard web server.

    Usage:
        maestro memory serve [--port PORT] [--host HOST] [--db DATABASE]
    """
    from .dashboard import create_dashboard_app

    # Get database path from args or use default
    database_path = None
    if args.db:
        database_path = Path(args.db)
    else:
        # Use default Maestro database location
        database_path = Path.home() / ".maestro" / "maestro.db"

    # Ensure database directory exists
    database_path.parent.mkdir(parents=True, exist_ok=True)

    # Create the FastAPI app
    app = create_dashboard_app(database_path=database_path, debug=args.debug)

    # Log startup info
    logger.info(f"Starting Maestro Memory Dashboard on {args.host}:{args.port}")
    logger.info(f"Database: {database_path}")
    logger.info(f"Dashboard URL: http://{args.host}:{args.port}")
    logger.info(f"API Documentation: http://{args.host}:{args.port}/api/docs")

    # Run the server
    try:
        uvicorn.run(
            app,
            host=args.host,
            port=args.port,
            log_level="debug" if args.debug else "info",
            access_log=not args.quiet,
            reload=args.debug  # Auto-reload in debug mode
        )
    except Exception as e:
        logger.error(f"Failed to start server: {e}")
        return 1

    return 0


def status_command(args: argparse.Namespace) -> int:
    """
    Show Maestro memory system status.

    Usage:
        maestro memory status [--db DATABASE]
    """
    from .service import MaestroMemoryService

    # Get database path
    database_path = None
    if args.db:
        database_path = Path(args.db)
    else:
        database_path = Path.home() / ".maestro" / "maestro.db"

    if not database_path.exists():
        logger.error(f"Database not found: {database_path}")
        logger.info("Run any Maestro command to initialize the memory system.")
        return 1

    # Initialize service and get stats
    import asyncio

    async def get_stats() -> int:
        service = MaestroMemoryService(database_path=database_path)
        await service.initialize()

        try:
            from .database.models import MaestroProject, MaestroTrack
            from sqlalchemy import select, func, text

            async with service.db_manager.get_async_session() as session:
                # Count projects
                project_result = await session.execute(
                    select(func.count()).select_from(MaestroProject)  # pylint: disable=not-callable
                )
                total_projects = project_result.scalar() or 0

                # Count tracks
                track_result = await session.execute(
                    select(func.count()).select_from(MaestroTrack)  # pylint: disable=not-callable
                )
                total_tracks = track_result.scalar() or 0

                # Count memories
                memory_result = await session.execute(
                    select(func.count()).select_from(text("memories")).where(text("is_active = 1"))  # pylint: disable=not-callable
                )
                total_memories = memory_result.scalar() or 0

                print(f"\n{'='*60}")
                print(f"  Maestro Memory System Status")
                print(f"{'='*60}")
                print(f"  Database: {database_path}")
                print(f"  Total Projects: {total_projects}")
                print(f"  Total Tracks: {total_tracks}")
                print(f"  Total Memories: {total_memories}")
                print(f"{'='*60}\n")

        finally:
            await service.close()
        
        return 0

    return asyncio.run(get_stats())


def scan_command(args: argparse.Namespace) -> int:
    """
    Scan filesystem for Maestro projects and populate the database.

    Usage:
        maestro memory scan [--dir DIR] [--depth DEPTH]
    """
    from .scanner import scan_projects
    import asyncio

    # Get directories to scan
    base_dirs = [args.dir] if args.dir else None

    async def run_scan() -> int:
        logger.info(f"Scanning for Maestro projects...")
        results = await scan_projects(base_dirs=base_dirs)

        print(f"\n{'='*60}")
        print(f"  Maestro Project Scan Results")
        print(f"{'='*60}")
        print(f"  Projects found: {results['projects_found']}")
        print(f"  Tracks found: {results['tracks_found']}")

        if results['projects']:
            print(f"\n  Discovered Projects:")
            for p in results['projects']:
                print(f"    - {p['name']} ({p['type']}) at {p['path']}")

        if results['tracks']:
            print(f"\n  Discovered Tracks:")
            for t in results['tracks']:
                print(f"    - {t['track_id']}: {t['title']}")

        if results['errors']:
            print(f"\n  Errors:")
            for e in results['errors']:
                print(f"    ✗ {e}")

        print(f"{'='*60}\n")
        return 0 if results['success'] else 1

    return asyncio.run(run_scan())


def migrate_command(args: argparse.Namespace) -> int:
    """
    Migrate memories from Memori database to Nexus.

    Usage:
        maestro memory migrate <source> [--db DATABASE] [--backup BACKUP_DIR]
    """
    import asyncio
    from .service import MaestroMemoryService
    from .migrations.memori_migration import (
        migrate_memori_to_nexus,
        validate_memori_database,
        get_migration_summary
    )

    # Get database paths
    source_db = Path(args.source)
    if args.db:
        target_db = Path(args.db)
    else:
        target_db = Path.home() / ".maestro" / "maestro.db"

    # Validate source database
    logger.info(f"Validating source database: {source_db}")
    is_valid, error_msg = validate_memori_database(str(source_db))
    if not is_valid:
        logger.error(f"Invalid Memori database: {error_msg}")
        return 1

    # Ensure target database directory exists
    target_db.parent.mkdir(parents=True, exist_ok=True)

    # Run migration
    async def run_migration() -> int:
        # Progress callback
        async def progress_callback(stage: str, progress: float, message: str) -> None:
            logger.info(f"[{stage}] {progress*100:.1f}%: {message}")

        # Initialize Nexus service
        service = MaestroMemoryService(database_path=target_db)
        await service.initialize()

        try:
            # Perform migration
            result = await migrate_memori_to_nexus(
                memori_db_path=str(source_db),
                nexus_service=service,
                backup_path=args.backup,
                progress_callback=progress_callback
            )

            # Print summary
            print("\n" + get_migration_summary(result))

            return 0 if result["success"] else 1

        finally:
            await service.close()

    return asyncio.run(run_migration())


def main() -> None:
    """
    Main CLI entry point.

    Usage:
        python -m maestro.memory.cli serve [--port PORT] [--host HOST]
        python -m maestro.memory.cli status [--db DATABASE]
    """
    parser = argparse.ArgumentParser(
        prog="maestro memory",
        description="Maestro Memory System - Command-line interface"
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    # `serve` command
    serve_parser = subparsers.add_parser(
        "serve",
        help="Start the Maestro Memory Dashboard web server"
    )
    serve_parser.add_argument(
        "--port", "-p",
        type=int,
        default=18765,
        help="Port to run the dashboard on (default: 18765)"
    )
    serve_parser.add_argument(
        "--host", "-H",
        type=str,
        default="127.0.0.1",
        help="Host to bind the dashboard to (default: 127.0.0.1)"
    )
    serve_parser.add_argument(
        "--db", "-d",
        type=str,
        default=None,
        help="Path to database file (default: ~/.maestro/maestro.db)"
    )
    serve_parser.add_argument(
        "--debug",
        action="store_true",
        help="Enable debug mode (verbose logging, auto-reload)"
    )
    serve_parser.add_argument(
        "--quiet", "-q",
        action="store_true",
        help="Suppress access logs"
    )

    # `status` command
    status_parser = subparsers.add_parser(
        "status",
        help="Show Maestro memory system status"
    )
    status_parser.add_argument(
        "--db", "-d",
        type=str,
        default=None,
        help="Path to database file (default: ~/.maestro/maestro.db)"
    )

    # `scan` command
    scan_parser = subparsers.add_parser(
        "scan",
        help="Scan filesystem for Maestro projects"
    )
    scan_parser.add_argument(
        "--dir", "-d",
        type=str,
        default=None,
        help="Directory to scan (default: ~/Prod)"
    )
    scan_parser.add_argument(
        "--depth",
        type=int,
        default=5,
        help="Maximum directory depth to scan (default: 5)"
    )

    # Parse arguments
    args = parser.parse_args()

    # Execute command
    if args.command == "serve":
        sys.exit(serve_command(args))
    elif args.command == "status":
        sys.exit(status_command(args))
    elif args.command == "scan":
        sys.exit(scan_command(args))
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
