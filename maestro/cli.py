"""
Maestro CLI - Main entry point for the Maestro framework.

This CLI provides access to all Maestro functionality including:
- Memory system operations (serve, status, migrate)
- TUI operations
- Future: project setup, track management, etc.
"""

import sys
import argparse


def main():
    """
    Main CLI entry point for Maestro.

    Usage:
        maestro memory <command> [options]
        maestro tui <command> [options]
    """
    parser = argparse.ArgumentParser(
        prog="maestro",
        description="Maestro - Spec-driven development framework for AI-assisted software engineering"
    )

    subparsers = parser.add_subparsers(dest="module", help="Maestro modules")

    # Memory module commands
    memory_parser = subparsers.add_parser(
        "memory",
        help="Maestro Memory System operations"
    )
    memory_subparsers = memory_parser.add_subparsers(dest="command", help="Memory commands")

    # `memory serve` command
    serve_parser = memory_subparsers.add_parser(
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

    # `memory status` command
    status_parser = memory_subparsers.add_parser(
        "status",
        help="Show Maestro memory system status"
    )
    status_parser.add_argument(
        "--db", "-d",
        type=str,
        default=None,
        help="Path to database file (default: ~/.maestro/maestro.db)"
    )

    # `memory migrate` command
    migrate_parser = memory_subparsers.add_parser(
        "migrate",
        help="Migrate memories from Memori database to Nexus"
    )
    migrate_parser.add_argument(
        "source",
        type=str,
        help="Path to Memori database file to migrate"
    )
    migrate_parser.add_argument(
        "--db", "-d",
        type=str,
        default=None,
        help="Path to target Nexus database (default: ~/.maestro/maestro.db)"
    )
    migrate_parser.add_argument(
        "--backup", "-b",
        type=str,
        default=None,
        help="Path to backup directory (default: no backup)"
    )

    # Parse arguments
    args = parser.parse_args()

    # Route to appropriate module
    if args.module == "memory":
        # Import and delegate to memory CLI
        from .memory.cli import serve_command, status_command, migrate_command

        # Create a namespace object for the command
        class Args:
            def __init__(self, **kwargs):
                for k, v in kwargs.items():
                    setattr(self, k, v)

        # Execute command
        if args.command == "serve":
            cmd_args = Args(
                port=args.port,
                host=args.host,
                db=args.db,
                debug=args.debug,
                quiet=args.quiet
            )
            sys.exit(serve_command(cmd_args))
        elif args.command == "status":
            cmd_args = Args(db=args.db)
            sys.exit(status_command(cmd_args))
        elif args.command == "migrate":
            cmd_args = Args(
                source=args.source,
                db=args.db,
                backup=args.backup
            )
            sys.exit(migrate_command(cmd_args))
        else:
            memory_parser.print_help()
            sys.exit(1)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
