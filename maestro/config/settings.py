"""
Maestro Configuration Settings Module

This module provides centralized configuration management for the Maestro framework.
It loads settings from defaults.yaml, applies environment variable overrides,
and validates the configuration structure.
"""

import os
import copy
import logging
import threading
from pathlib import Path
from typing import TYPE_CHECKING, Any, Optional, TypeVar, Union, cast, overload
from dataclasses import dataclass, field

try:
    import yaml  # type: ignore[import-untyped]
except ImportError:
    yaml = None

if TYPE_CHECKING:
    from pydantic import BaseModel, Field, field_validator, ConfigDict, ValidationError
    PYDANTIC_AVAILABLE = True
else:
    try:
        from pydantic import BaseModel, Field, field_validator, ConfigDict, ValidationError
        PYDANTIC_AVAILABLE = True
    except ImportError:
        PYDANTIC_AVAILABLE = False
        BaseModel = object
        ConfigDict = None
        ValidationError = Exception


# Module logger for configuration errors
_logger = logging.getLogger(__name__)


# Constants for validation
MAX_YAML_SIZE_BYTES = 1 * 1024 * 1024  # 1MB size limit for YAML files
MIN_PORT = 1
MAX_PORT = 65535
DEFAULT_DASHBOARD_PORT = 18765


# Default configuration paths
DEFAULT_HOME = Path.home() / ".maestro"
DEFAULT_CONFIG_PATH = Path(__file__).parent / "defaults.yaml"


def expand_path(path: Union[str, Path]) -> Path:
    """Expand user home directory and resolve path."""
    path_str = str(path)
    if path_str.startswith("~"):
        path_str = os.path.expanduser(path_str)
    return Path(path_str).resolve()


def get_env_bool(key: str, default: bool = False) -> bool:
    """Get boolean value from environment variable."""
    value = os.getenv(key, "").lower()
    if not value:
        return default
    return value in ("true", "1", "yes", "on")


def get_env_int(key: str, default: int = 0) -> int:
    """Get integer value from environment variable."""
    try:
        return int(os.getenv(key, str(default)))
    except ValueError:
        return default


def validate_port(port: Any, field_name: str = "port") -> int:
    """
    Validate that a port number is within the valid range.

    Args:
        port: The port value to validate
        field_name: Name of the field for error messages

    Returns:
        Validated port as integer

    Raises:
        ValueError: If port is outside valid range
        TypeError: If port cannot be converted to int
    """
    try:
        port_int = int(port)
    except (TypeError, ValueError) as e:
        raise TypeError(f"{field_name} must be an integer, got {type(port).__name__}") from e

    if not (MIN_PORT <= port_int <= MAX_PORT):
        raise ValueError(
            f"{field_name} must be between {MIN_PORT} and {MAX_PORT}, got {port_int}"
        )
    return port_int


def validate_path(path: Any, field_name: str = "path", must_exist: bool = False) -> Path:
    """
    Validate a path value.

    Args:
        path: The path value to validate
        field_name: Name of the field for error messages
        must_exist: Whether the path must exist on the filesystem

    Returns:
        Validated Path object

    Raises:
        TypeError: If path is not a string or Path
        ValueError: If path is empty or doesn't exist when must_exist=True
    """
    if path is None:
        raise ValueError(f"{field_name} cannot be None")

    if not isinstance(path, (str, Path)):
        raise TypeError(f"{field_name} must be a string or Path, got {type(path).__name__}")

    path_str = str(path).strip()
    if not path_str:
        raise ValueError(f"{field_name} cannot be empty")

    result = Path(path_str)

    if must_exist and not result.exists():
        raise ValueError(f"{field_name} does not exist: {result}")

    return result


def get_env_str(key: str, default: str = "") -> str:
    """Get string value from environment variable."""
    return os.getenv(key, default)


if PYDANTIC_AVAILABLE:

    class CoreSettings(BaseModel):
        """Core framework settings."""
        version: str = "2.0.0"
        environment: str = "production"
        paths: dict = Field(default_factory=lambda: {
            "home": "~/.maestro",
            "project_dir": ".maestro",
            "tracks_dir": "tracks",
            "data_dir": "data",
        })
        debug: bool = False
        verbosity: int = 1
        profile: str = "production"

        @field_validator('paths', mode='before')
        @classmethod
        def expand_paths(cls, v: Any) -> Any:
            """Expand paths to absolute paths."""
            if isinstance(v, dict):
                result = {}
                for key, value in v.items():
                    result[key] = str(expand_path(value))
                return result
            return v

    class MemoryDatabaseSettings(BaseModel):
        """Memory database settings."""
        path: str = "~/.maestro/memory.db"
        pool_size: int = 5
        max_overflow: int = 10
        pool_timeout: int = 30
        foreign_keys: bool = True
        wal_mode: bool = True

        @field_validator('path', mode='before')
        @classmethod
        def expand_path(cls, v: Any) -> str:
            return str(expand_path(v))

    class MemoryHooksSettings(BaseModel):
        """Memory hooks settings."""
        native: dict = Field(default_factory=lambda: {
            "enabled": True,
            "process_monitor": {"enabled": True, "sampling_interval": 1.0, "memory_threshold": 0.85},
            "inactivity_detector": {"enabled": True, "threshold_seconds": 30},
            "persistent_buffer": {"enabled": True, "buffer_size": 1000, "flush_interval": 5},
        })
        coordination: dict = Field(default_factory=lambda: {
            "enabled": True,
            "file_claims": {"enabled": True, "claim_ttl": 3600, "conflict_resolution": "error"},
            "handoffs": {"enabled": True, "auto_create": False, "storage_path": "~/.maestro/handoffs"},
            "ledgers": {"enabled": True, "auto_update": True, "storage_path": "~/.maestro/ledgers"},
        })

    class MemoryEmbeddingsSettings(BaseModel):
        """Memory embeddings settings."""
        enabled: bool = True
        model: str = "sentence-transformers/all-MiniLM-L6-v2"
        dimensions: int = 384
        batch_size: int = 32
        similarity_threshold: float = 0.75
        max_results: int = 10

    class MemoryCategory(BaseModel):
        """Memory category definition."""
        name: str
        description: str
        ttl: Optional[int] = None

    class MemorySettings(BaseModel):
        """Memory system settings."""
        enabled: bool = True
        database: "MemoryDatabaseSettings" = Field(default_factory=MemoryDatabaseSettings)
        hooks: "MemoryHooksSettings" = Field(default_factory=MemoryHooksSettings)
        embeddings: "MemoryEmbeddingsSettings" = Field(default_factory=MemoryEmbeddingsSettings)
        categories: list = Field(default_factory=list)

    class TracksStorageSettings(BaseModel):
        """Tracks storage settings."""
        base_path: str = "tracks"
        dir_format: str = "{date}_{name}"
        date_format: str = "%Y%m%d"

    class TracksGitSettings(BaseModel):
        """Tracks git integration settings."""
        enabled: bool = True
        auto_commit: bool = True
        commit_format: str = "maestro(track): {track_name} - {action}"
        branch_format: str = "maestro/{track_name}"
        auto_push: bool = False

    class TracksMemorySettings(BaseModel):
        """Tracks memory integration settings."""
        enabled: bool = True
        link_memories: bool = True
        store_context: bool = True
        namespace: str = "tracks"

    class TracksPhase(BaseModel):
        """Track phase definition."""
        name: str
        description: str
        required: bool = True

    class TracksSettings(BaseModel):
        """Tracks system settings."""
        storage: "TracksStorageSettings" = Field(default_factory=TracksStorageSettings)
        git: "TracksGitSettings" = Field(default_factory=TracksGitSettings)
        memory: "TracksMemorySettings" = Field(default_factory=TracksMemorySettings)
        phases: list = Field(default_factory=list)

    class SkillCategory(BaseModel):
        """Skill category definition."""
        name: str
        description: str
        skills: list

    class SkillsActivationSettings(BaseModel):
        """Skills activation settings."""
        min_confidence: float = 0.7
        allow_multiple: bool = True
        require_confirmation: bool = False
        blocked_combinations: list = Field(default_factory=list)

    class SkillsSettings(BaseModel):
        """Skills system settings."""
        total_count: int = 109
        auto_suggest: bool = True
        max_suggestions: int = 3
        categories: dict = Field(default_factory=dict)
        activation: "SkillsActivationSettings" = Field(default_factory=SkillsActivationSettings)

    class AgentSettings(BaseModel):
        """Agent definition."""
        name: str
        description: str
        model: str = "opus"
        permissions: list = Field(default_factory=list)

    class AgentsDefaults(BaseModel):
        """Default agent settings."""
        model: str = "opus"
        max_iterations: int = 10
        timeout: int = 300
        cache_enabled: bool = True

    class AgentsSelectionSettings(BaseModel):
        """Agent selection settings."""
        complexity: dict = Field(default_factory=lambda: {
            "low": 3, "medium": 7, "high": 15
        })
        by_task: dict = Field(default_factory=dict)

    class AgentsSettings(BaseModel):
        """Agents system settings."""
        total_count: int = 32
        defaults: "AgentsDefaults" = Field(default_factory=AgentsDefaults)
        categories: dict = Field(default_factory=dict)
        selection: "AgentsSelectionSettings" = Field(default_factory=AgentsSelectionSettings)

    class TldrLayer(BaseModel):
        """TLDR analysis layer."""
        name: str
        description: str
        enabled: bool = True

    class TldrSemanticSettings(BaseModel):
        """TLDR semantic index settings."""
        enabled: bool = True
        path: str = "~/.maestro/tldr-index"
        update_strategy: str = "auto"
        max_file_size: int = 1048576

        @field_validator('path', mode='before')
        @classmethod
        def expand_path(cls, v: Any) -> str:
            return str(expand_path(v))

    class TldrDaemonSettings(BaseModel):
        """TLDR daemon settings."""
        enabled: bool = False
        port: int = 18766
        host: str = "127.0.0.1"

    class TldrSettings(BaseModel):
        """TLDR code analysis settings."""
        enabled: bool = True
        layers: list = Field(default_factory=list)
        semantic_index: "TldrSemanticSettings" = Field(default_factory=TldrSemanticSettings)
        daemon: "TldrDaemonSettings" = Field(default_factory=TldrDaemonSettings)
        commands: dict = Field(default_factory=dict)

    class CriticalThinkIntegrationPoint(BaseModel):
        """Critical Think integration point."""
        enabled: bool = True
        confidence_threshold: int = 7

    class CriticalThinkThresholds(BaseModel):
        """Critical Think confidence thresholds."""
        critical: int = 4
        warning: int = 6
        acceptable: int = 7
        high: int = 9

    class CriticalThinkOutputSettings(BaseModel):
        """Critical Think output settings."""
        format: str = "detailed"
        show_confidence: bool = True
        show_all_steps: bool = True
        show_risks: bool = True
        highlight_pitfalls: bool = True
        show_revised_confidence: bool = True

    class CriticalThinkBehaviorSettings(BaseModel):
        """Critical Think behavior settings."""
        auto_proceed: bool = True
        require_confirmation_on_low_confidence: bool = True
        max_assumptions: int = 3
        max_risks: int = 3
        verbose: bool = True

    class CriticalThinkSettings(BaseModel):
        """Critical Think settings."""
        enabled: bool = True
        thresholds: "CriticalThinkThresholds" = Field(default_factory=CriticalThinkThresholds)
        integration_points: dict = Field(default_factory=dict)
        output: "CriticalThinkOutputSettings" = Field(default_factory=CriticalThinkOutputSettings)
        behavior: "CriticalThinkBehaviorSettings" = Field(default_factory=CriticalThinkBehaviorSettings)
        templates: dict = Field(default_factory=dict)

    class DashboardServerSettings(BaseModel):
        """Dashboard server settings."""
        host: str = "127.0.0.1"
        port: int = Field(default=DEFAULT_DASHBOARD_PORT, ge=MIN_PORT, le=MAX_PORT)
        auto_start: bool = True
        open_browser: bool = True

        @field_validator('port')
        @classmethod
        def validate_port_range(cls, v: Any) -> int:
            """Validate port is in valid range."""
            return validate_port(v, "port")

    class DashboardAuthSettings(BaseModel):
        """Dashboard authentication settings."""
        enabled: bool = False
        type: str = "token"
        token: Optional[str] = None

    class DashboardFeaturesSettings(BaseModel):
        """Dashboard feature settings."""
        memory_browser: dict = Field(default_factory=lambda: {"enabled": True})
        coordination: dict = Field(default_factory=lambda: {
            "enabled": True,
            "show_file_claims": True,
            "show_handoffs": True,
            "show_ledgers": True,
        })
        tracks: dict = Field(default_factory=lambda: {"enabled": True})
        agents: dict = Field(default_factory=lambda: {"enabled": True})
        analytics: dict = Field(default_factory=lambda: {"enabled": True})

    class DashboardSettings(BaseModel):
        """Dashboard settings."""
        enabled: bool = True
        server: "DashboardServerSettings" = Field(default_factory=DashboardServerSettings)
        features: "DashboardFeaturesSettings" = Field(default_factory=DashboardFeaturesSettings)
        auth: "DashboardAuthSettings" = Field(default_factory=DashboardAuthSettings)

    class CliSettings(BaseModel):
        """CLI settings."""
        prefix: str = "maestro:"
        color: bool = True
        progress: bool = True
        pager: str = "auto"
        editor: Optional[str] = None
        completion: dict = Field(default_factory=lambda: {
            "enabled": True,
            "shell": "auto",
        })

    class MaestroSettings(BaseModel):  # type: ignore[no-redef]
        """Root Maestro settings model."""
        core: CoreSettings = Field(default_factory=CoreSettings)
        memory: MemorySettings = Field(default_factory=MemorySettings)
        tracks: TracksSettings = Field(default_factory=TracksSettings)
        skills: SkillsSettings = Field(default_factory=SkillsSettings)
        agents: AgentsSettings = Field(default_factory=AgentsSettings)
        tldr: TldrSettings = Field(default_factory=TldrSettings)
        critical_think: CriticalThinkSettings = Field(default_factory=CriticalThinkSettings)
        dashboard: DashboardSettings = Field(default_factory=DashboardSettings)
        cli: CliSettings = Field(default_factory=CliSettings)

        model_config = ConfigDict(validate_assignment=True)


@dataclass
class SimpleSettings:
    """Fallback settings class when Pydantic is not available."""

    # Core settings
    version: str = "2.0.0"
    environment: str = "production"
    home_path: Path = field(default_factory=lambda: DEFAULT_HOME)

    # Memory settings
    memory_enabled: bool = True
    memory_db_path: Path = field(default_factory=lambda: DEFAULT_HOME / "memory.db")

    # Dashboard settings
    dashboard_enabled: bool = True
    dashboard_host: str = "127.0.0.1"
    dashboard_port: int = 18765

    # Debug settings
    debug: bool = False
    verbosity: int = 1

    # CLI settings
    cli_prefix: str = "maestro:"

    def get(self, key: str, default: Any = None) -> Any:
        """Get a configuration value by dot-separated key."""
        keys = key.split(".")
        value = self
        for k in keys:
            if hasattr(value, k):
                value = getattr(value, k)
            else:
                return default
        return value


class SettingsManager:
    """
    Configuration manager for Maestro framework.

    Loads settings from defaults.yaml, applies environment variable overrides,
    and provides typed access to configuration values.
    """

    def __init__(self, config_path: Optional[Path] = None):
        """
        Initialize the settings manager.

        Args:
            config_path: Path to configuration file. If None, uses default.
        """
        self._config_path = config_path or DEFAULT_CONFIG_PATH
        self._raw_config: dict = {}
        self._settings: Optional[Union["MaestroSettings", SimpleSettings]] = None
        self._load_config()

    def _load_config(self) -> None:
        """Load configuration from YAML file."""
        if yaml is None:
            _logger.error(
                "PyYAML is required for configuration loading. "
                "Install it with: pip install pyyaml"
            )
            raise ImportError(
                "PyYAML is required for configuration loading. "
                "Install it with: pip install pyyaml"
            )

        # Load default configuration
        if self._config_path.exists():
            # Check file size before loading to prevent DOS attacks
            file_size = self._config_path.stat().st_size
            if file_size > MAX_YAML_SIZE_BYTES:
                _logger.error(
                    f"Configuration file exceeds size limit: "
                    f"{file_size} bytes (max: {MAX_YAML_SIZE_BYTES} bytes)"
                )
                raise ValueError(
                    f"Configuration file exceeds maximum size of "
                    f"{MAX_YAML_SIZE_BYTES} bytes"
                )

            try:
                with open(self._config_path, "r", encoding="utf-8") as f:
                    self._raw_config = yaml.safe_load(f) or {}
            except yaml.YAMLError as e:
                _logger.error(f"Failed to parse YAML configuration: {e}")
                raise ValueError(f"Invalid YAML in configuration file: {e}") from e
            except (OSError, IOError) as e:
                _logger.error(f"Failed to read configuration file: {e}")
                raise
        else:
            # Use minimal default config if file doesn't exist
            _logger.debug(f"Configuration file not found: {self._config_path}, using defaults")
            self._raw_config = self._get_minimal_config()

        # Apply environment overrides
        self._apply_env_overrides()

        # Create typed settings
        self._create_typed_settings()

    def _get_minimal_config(self) -> dict:
        """Get minimal configuration when defaults.yaml is not available."""
        return {
            "core": {
                "version": "2.0.0",
                "environment": "production",
                "paths": {"home": "~/.maestro"},
                "debug": False,
                "verbosity": 1,
                "profile": "production",
            },
            "memory": {"enabled": True, "database": {"path": "~/.maestro/memory.db"}},
            "dashboard": {
                "enabled": True,
                "server": {"host": "127.0.0.1", "port": 18765},
            },
            "cli": {"prefix": "maestro:"},
        }

    def _apply_env_overrides(self) -> None:
        """Apply environment variable overrides to configuration."""
        # Core overrides
        if "MAESTRO_HOME" in os.environ:
            self._set_nested("core.paths.home", os.environ["MAESTRO_HOME"])

        if "MAESTRO_DEBUG" in os.environ:
            self._set_nested("core.debug", get_env_bool("MAESTRO_DEBUG"))

        if "MAESTRO_PROFILE" in os.environ:
            self._set_nested("core.profile", os.environ["MAESTRO_PROFILE"])

        # Memory overrides
        if "MAESTRO_MEMORY_PATH" in os.environ:
            self._set_nested("memory.database.path", os.environ["MAESTRO_MEMORY_PATH"])

        # Dashboard overrides
        if "MAESTRO_DASHBOARD_HOST" in os.environ:
            self._set_nested("dashboard.server.host", os.environ["MAESTRO_DASHBOARD_HOST"])

        if "MAESTRO_DASHBOARD_PORT" in os.environ:
            self._set_nested("dashboard.server.port", get_env_int("MAESTRO_DASHBOARD_PORT", 18765))

        # Tracks overrides
        if "MAESTRO_TRACKS_DIR" in os.environ:
            self._set_nested("tracks.storage.base_path", os.environ["MAESTRO_TRACKS_DIR"])

    def _set_nested(self, path: str, value: Any) -> None:
        """Set a nested configuration value using dot-separated path."""
        keys = path.split(".")
        current = self._raw_config
        for key in keys[:-1]:
            if key not in current:
                current[key] = {}
            current = current[key]
        current[keys[-1]] = value

    def _create_typed_settings(self) -> None:
        """Create typed settings from raw configuration."""
        if PYDANTIC_AVAILABLE:
            try:
                self._settings = MaestroSettings(**self._raw_config)
                _logger.debug("Successfully created typed settings from configuration")
            except ValidationError as e:
                # Log validation errors with details
                _logger.warning(f"Configuration validation failed: {e}")
                _logger.debug(f"Validation errors: {e.errors()}")
                # Fall back to simple settings if validation fails
                self._settings = SimpleSettings()
                _logger.info("Falling back to SimpleSettings due to validation error")
            except Exception as e:
                # Catch any other unexpected errors
                _logger.error(f"Unexpected error creating typed settings: {type(e).__name__}: {e}")
                # Fall back to simple settings
                self._settings = SimpleSettings()
                _logger.info("Falling back to SimpleSettings due to unexpected error")
        else:
            _logger.debug("Pydantic not available, using SimpleSettings")
            self._settings = SimpleSettings()

    def reload(self) -> None:
        """Reload configuration from file."""
        self._load_config()

    @property
    def settings(self) -> Union["MaestroSettings", SimpleSettings]:
        """Get the typed settings object."""
        if self._settings is None:
            self._create_typed_settings()
        assert self._settings is not None
        return cast(Union[MaestroSettings, SimpleSettings], self._settings)

    @property
    def raw(self) -> dict:
        """Get the raw configuration dictionary."""
        return self._raw_config

    _T = TypeVar("_T")

    @overload
    def get(self, key: str, default: _T) -> _T: ...

    @overload
    def get(self, key: str, default: None = None) -> Any: ...

    def get(self, key: str, default: Any = None) -> Any:
        """
        Get a configuration value by dot-separated key.

        Args:
            key: Dot-separated configuration key (e.g., "core.version")
            default: Default value if key not found

        Returns:
            Configuration value or default
        """
        keys = key.split(".")
        value = self._raw_config
        for k in keys:
            if isinstance(value, dict) and k in value:
                value = value[k]
            else:
                return default
        return value

    # Convenience properties for common settings
    @property
    def version(self) -> str:
        """Get Maestro version."""
        return self.get("core.version", "2.0.0")

    @property
    def debug(self) -> bool:
        """Get debug mode setting."""
        return self.get("core.debug", False)

    @property
    def home_path(self) -> Path:
        """Get Maestro home directory path."""
        path = self.get("core.paths.home", "~/.maestro")
        return expand_path(path)

    @property
    def tracks_dir(self) -> Path:
        """Get tracks directory path."""
        path = self.get("tracks.storage.base_path", "tracks")
        return expand_path(path)

    @property
    def memory_db_path(self) -> Path:
        """Get memory database path."""
        path = self.get("memory.database.path", "~/.maestro/memory.db")
        return expand_path(path)

    @property
    def dashboard_host(self) -> str:
        """Get dashboard host."""
        return self.get("dashboard.server.host", "127.0.0.1")

    @property
    def dashboard_port(self) -> int:
        """Get dashboard port."""
        return self.get("dashboard.server.port", 18765)

    @property
    def cli_prefix(self) -> str:
        """Get CLI command prefix."""
        return self.get("cli.prefix", "maestro:")

    @property
    def memory_enabled(self) -> bool:
        """Check if memory system is enabled."""
        return self.get("memory.enabled", True)

    @property
    def dashboard_enabled(self) -> bool:
        """Check if dashboard is enabled."""
        return self.get("dashboard.enabled", True)

    @property
    def skills_enabled(self) -> bool:
        """Check if skills are enabled."""
        return self.get("skills.auto_suggest", True)

    @property
    def tldr_enabled(self) -> bool:
        """Check if TLDR code analysis is enabled."""
        return self.get("tldr.enabled", True)

    @property
    def critical_think_enabled(self) -> bool:
        """Check if Critical Think is enabled."""
        return self.get("critical_think.enabled", True)

    # Agent access methods
    def get_agent(self, name: str) -> Optional[dict]:
        """Get agent configuration by name."""
        categories: dict[str, Any] = self.get("agents.categories", {})
        if not isinstance(categories, dict):
            return None
        for category in categories.values():
            if not isinstance(category, dict):
                continue
            agents = category.get("agents", [])
            if not isinstance(agents, list):
                continue
            for agent in agents:
                if not isinstance(agent, dict):
                    continue
                if agent.get("name") == name:
                    return agent.copy()
        return None

    def list_agents(self, category: Optional[str] = None) -> list:
        """List all agents or agents in a specific category."""
        if category:
            cat_config = self.get(f"agents.categories.{category}")
            return cat_config.get("agents", []) if cat_config else []
        else:
            agents = []
            for cat_config in self.get("agents.categories", {}).values():
                agents.extend(cat_config.get("agents", []))
            return agents

    # Skill access methods
    def get_skill(self, name: str) -> Optional[dict]:
        """Get skill configuration by name."""
        categories: dict[str, Any] = self.get("skills.categories", {})
        if not isinstance(categories, dict):
            return None
        for cat_name, cat_config in categories.items():
            if not isinstance(cat_config, dict):
                continue
            skills = cat_config.get("skills", [])
            if isinstance(skills, list) and name in skills:
                return {
                    "name": name,
                    "category": cat_name,
                    "description": cat_config.get("description", ""),
                }
        return None

    def list_skills(self, category: Optional[str] = None) -> list:
        """List all skills or skills in a specific category."""
        if category:
            cat_config = self.get(f"skills.categories.{category}")
            return cat_config.get("skills", []) if cat_config else []
        else:
            skills = []
            for cat_config in self.get("skills.categories", {}).values():
                skills.extend(cat_config.get("skills", []))
            return skills


# Global settings instance with thread-safe initialization
_global_settings: Optional[SettingsManager] = None
_global_settings_lock = threading.Lock()
_global_settings_initialized = False


def get_settings(config_path: Optional[Path] = None) -> SettingsManager:
    """
    Get the global settings manager instance.

    This function is thread-safe and uses proper locking to ensure
    only one SettingsManager instance is created. The config_path
    is only used on first initialization; subsequent calls ignore it.

    Args:
        config_path: Optional path to configuration file (only used on first call)

    Returns:
        SettingsManager instance
    """
    global _global_settings, _global_settings_initialized

    # Fast path: return existing instance if already initialized
    if _global_settings_initialized and _global_settings is not None:
        return cast(SettingsManager, _global_settings)

    # Slow path: initialize with lock
    with _global_settings_lock:
        # Double-check after acquiring lock
        if _global_settings is None:
            _global_settings = SettingsManager(config_path)
        _global_settings_initialized = True

        assert _global_settings is not None
        return cast(SettingsManager, _global_settings)


def reload_settings() -> None:
    """
    Reload the global settings instance.

    This function is thread-safe. It resets the initialization flag
    so the next call will re-initialize the settings.
    """
    global _global_settings, _global_settings_initialized
    with _global_settings_lock:
        if _global_settings is not None:
            _global_settings.reload()
            # Reset flag to ensure consistent state
            _global_settings_initialized = True


# Convenience functions for accessing settings
def get_version() -> str:
    """Get Maestro version."""
    return get_settings().version


def get_home_path() -> Path:
    """Get Maestro home directory path."""
    return get_settings().home_path


def is_debug() -> bool:
    """Check if debug mode is enabled."""
    return get_settings().debug


def is_memory_enabled() -> bool:
    """Check if memory system is enabled."""
    return get_settings().memory_enabled


def is_dashboard_enabled() -> bool:
    """Check if dashboard is enabled."""
    return get_settings().dashboard_enabled


def get_cli_prefix() -> str:
    """Get CLI command prefix."""
    return get_settings().cli_prefix
