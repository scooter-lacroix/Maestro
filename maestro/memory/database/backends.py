"""
Unified Storage Backend for Maestro.
Combines SQLite for OLTP and DuckDB for OLAP.
"""

import os
import logging
import sqlite3
from typing import Optional
from sqlalchemy import create_engine, text
from sqlalchemy.orm import sessionmaker, Session
import duckdb

logger = logging.getLogger(__name__)

class UnifiedStorageBackend:
    """
    Manages both SQLite (OLTP) and DuckDB (OLAP) storage layers.
    """

    def __init__(
        self,
        db_path: Optional[str] = None,
        duckdb_path: Optional[str] = None,
        echo: bool = False
    ):
        if db_path is None:
            db_path = os.path.expanduser("~/.maestro/memory.db")
        if duckdb_path is None:
            duckdb_path = os.path.expanduser("~/.maestro/analytics.duckdb")

        self.db_path = db_path
        self.duckdb_path = duckdb_path
        self.echo = echo

        self.sqlite_engine = None
        self.SessionLocal = None
        self.duckdb_conn = None

    def initialize(self) -> None:
        """Initialize both database engines."""
        # Ensure directory exists
        os.makedirs(os.path.dirname(os.path.abspath(self.db_path)), exist_ok=True)
        os.makedirs(os.path.dirname(os.path.abspath(self.duckdb_path)), exist_ok=True)

        # Validate db_path to prevent SQL injection (no quotes allowed)
        if "'" in self.db_path or '"' in self.db_path:
            raise ValueError(f"Invalid database path (contains quotes): {self.db_path}")

        # 1. Initialize SQLite
        sqlite_url = f"sqlite:///{self.db_path}"
        self.sqlite_engine = create_engine(
            sqlite_url,
            echo=self.echo,
            connect_args={"check_same_thread": False}
        )

        # Enable WAL mode
        with self.sqlite_engine.begin() as conn:
            conn.execute(text("PRAGMA journal_mode=WAL"))
            conn.execute(text("PRAGMA foreign_keys=ON"))

        self.SessionLocal = sessionmaker(
            bind=self.sqlite_engine,
            expire_on_commit=False
        )

        # 2. Initialize DuckDB
        self.duckdb_conn = duckdb.connect(self.duckdb_path)

        # Install and load sqlite extension (handles offline gracefully)
        try:
            self.duckdb_conn.execute("INSTALL sqlite;")
        except Exception as e:
            logger.warning(f"Could not install sqlite extension (may already exist or offline): {e}")

        try:
            self.duckdb_conn.execute("LOAD sqlite;")
        except Exception as e:
            logger.error(f"Could not load sqlite extension: {e}")
            raise

        # Attach SQLite database to DuckDB for OLAP
        # Path is validated above to prevent injection
        self.duckdb_conn.execute(f"ATTACH '{self.db_path}' AS memory (TYPE SQLITE);")

        logger.info(f"UnifiedStorageBackend initialized with SQLite: {self.db_path} and DuckDB: {self.duckdb_path}")

    def get_session(self) -> Session:
        """Get a new SQLAlchemy session for SQLite."""
        if not self.SessionLocal:
            raise RuntimeError("Backend not initialized. Call initialize() first.")
        return self.SessionLocal()

    def query_analytics(self, query: str, parameters=None):
        """Execute a query via DuckDB (OLAP layer)."""
        if not self.duckdb_conn:
            raise RuntimeError("Backend not initialized. Call initialize() first.")
        return self.duckdb_conn.execute(query, parameters)

    def shutdown(self) -> None:
        """Clean shutdown of database connections."""
        if self.sqlite_engine:
            self.sqlite_engine.dispose()
            self.sqlite_engine = None

        if self.duckdb_conn:
            try:
                self.duckdb_conn.close()
            except Exception as e:
                logger.error(f"Error closing DuckDB connection: {e}")
            self.duckdb_conn = None

        self.SessionLocal = None
        logger.info("UnifiedStorageBackend shut down successfully.")
