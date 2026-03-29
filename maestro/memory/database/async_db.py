"""
Async database session manager for Maestro-owned queries.

This module keeps the async SQLAlchemy plumbing separate from the legacy
memory service so callers can migrate toward a thin bridge without importing
Nexus internals.
"""

from __future__ import annotations

from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any, AsyncIterator, Optional

from sqlalchemy import event
from sqlalchemy import create_engine
from sqlalchemy.ext.asyncio import AsyncEngine, AsyncSession, async_sessionmaker, create_async_engine
from sqlalchemy.orm import Session, sessionmaker


def _normalize_sqlite_url(database_url: str) -> str:
    if database_url.startswith("sqlite+aiosqlite://"):
        return database_url
    if database_url.startswith("sqlite:///"):
        return database_url.replace("sqlite:///", "sqlite+aiosqlite:///", 1)
    return database_url


class AsyncDatabaseManager:
    """Minimal async session manager for Maestro-owned SQLite queries."""

    def __init__(
        self,
        database_path: Optional[Path] = None,
        database_url: Optional[str] = None,
    ) -> None:
        if database_url is None:
            if database_path is None:
                database_path = Path.home() / ".maestro" / "maestro.db"
            database_path.parent.mkdir(parents=True, exist_ok=True)
            database_url = f"sqlite+aiosqlite:///{database_path}"

        self.database_url = _normalize_sqlite_url(database_url)
        self.engine: AsyncEngine | None = None
        self.session_factory: async_sessionmaker[AsyncSession] | None = None
        self.sync_engine: Any = None
        self.sync_session_factory: Any = None
        self._sync_fallback = False
        self._initialized = False

    async def initialize(self) -> None:
        if self._initialized:
            return

        try:
            self.engine = create_async_engine(
                self.database_url,
                future=True,
                pool_pre_ping=True,
            )
            self.session_factory = async_sessionmaker(
                bind=self.engine,
                expire_on_commit=False,
                class_=AsyncSession,
            )
            await self._setup_sqlite_pragmas()
        except ModuleNotFoundError as exc:
            if "aiosqlite" not in str(exc):
                raise
            sync_url = self.database_url.replace("sqlite+aiosqlite:///", "sqlite:///", 1)
            self.sync_engine = create_engine(sync_url, future=True)
            self.sync_session_factory = sessionmaker(
                bind=self.sync_engine,
                expire_on_commit=False,
                class_=Session,
            )
            self._sync_fallback = True
        self._initialized = True

    async def close(self) -> None:
        if self.engine is not None:
            await self.engine.dispose()
        if self.sync_engine is not None:
            self.sync_engine.dispose()
        self.engine = None
        self.session_factory = None
        self.sync_engine = None
        self.sync_session_factory = None
        self._sync_fallback = False
        self._initialized = False

    @asynccontextmanager
    async def get_async_session(self) -> AsyncIterator[AsyncSession]:
        await self.initialize()
        if self._sync_fallback:
            assert self.sync_session_factory is not None
            session = self.sync_session_factory()
            try:
                yield SyncSessionAdapter(session)
            finally:
                session.close()
            return
        assert self.session_factory is not None
        async with self.session_factory() as session:
            yield session

    async def _setup_sqlite_pragmas(self) -> None:
        if self.engine is None:
            return

        if not self.database_url.startswith("sqlite"):
            return

        @event.listens_for(self.engine.sync_engine, "connect")
        def _set_sqlite_pragmas(dbapi_connection, _connection_record) -> None:  # type: ignore[no-untyped-def]
            cursor = dbapi_connection.cursor()
            cursor.execute("PRAGMA foreign_keys=ON")
            cursor.execute("PRAGMA journal_mode=WAL")
            cursor.execute("PRAGMA synchronous=NORMAL")
            cursor.execute("PRAGMA busy_timeout=10000")
            cursor.close()

    @property
    def initialized(self) -> bool:
        return self._initialized

    @classmethod
    def from_database_path(cls, database_path: Path) -> "AsyncDatabaseManager":
        return cls(database_path=database_path)

    @classmethod
    def from_database_url(cls, database_url: str) -> "AsyncDatabaseManager":
        return cls(database_url=database_url)


class SyncSessionAdapter:
    """Small async-shaped wrapper for sync SQLAlchemy sessions."""

    def __init__(self, session: Session) -> None:
        self._session = session

    async def execute(self, *args: Any, **kwargs: Any) -> Any:
        return self._session.execute(*args, **kwargs)

    async def scalar(self, *args: Any, **kwargs: Any) -> Any:
        return self._session.scalar(*args, **kwargs)

    async def commit(self) -> None:
        self._session.commit()

    async def rollback(self) -> None:
        self._session.rollback()

    def __getattr__(self, name: str) -> Any:
        return getattr(self._session, name)


__all__ = ["AsyncDatabaseManager"]
