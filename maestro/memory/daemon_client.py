"""
Maestro Memory Daemon Client

Provides client functionality for interacting with the memory daemon.
Includes cross-platform fixes for session ID lookup, file existence checks,
PID management to prevent duplicate daemon spawns, and JSON-RPC/REST API
for state queries and cross-terminal coordination.
"""

import json
import os
import socket
import time
import yaml
from pathlib import Path
from typing import Optional, Dict, Any, List, Union
from loguru import logger
from urllib.parse import urljoin


class MemoryDaemonClient:
    """
    Client for interacting with the memory daemon.

    Handles cross-platform concerns including:
    - Session ID lookup with truncation support
    - File existence checks before path usage
    - PID-based daemon lifecycle management
    - JSON-RPC/REST API for state queries
    - Cross-terminal coordination
    - YAML goal/now/next schema support
    """

    def __init__(self, jsonl_path: str, pid_file: Optional[str] = None, api_base_url: Optional[str] = None,
                 yaml_schema_path: Optional[str] = None):
        """
        Initialize the memory daemon client.

        Args:
            jsonl_path: Path to the JSONL file containing session data
            pid_file: Optional path to PID file for daemon management
            api_base_url: Optional base URL for daemon API (default: http://localhost:8080)
            yaml_schema_path: Optional path to YAML schema for goal/now/next structures
        """
        self.jsonl_path = Path(jsonl_path)
        self.pid_file = Path(pid_file) if pid_file else None
        self.api_base_url = api_base_url or "http://localhost:8080"
        self.yaml_schema_path = Path(yaml_schema_path) if yaml_schema_path else None

        # Lazy import of requests for optional functionality
        try:
            import requests
            self.session = requests.Session()
            self.session.headers.update({'Content-Type': 'application/json'})
            self.requests_available = True
        except ImportError:
            self.session = None
            self.requests_available = False
        # Set default timeout for all requests
        self.session_timeout = 15  # seconds

    def lookup_session(self, session_id: str) -> Optional[Dict[str, Any]]:
        """
        Look up a session by ID, supporting truncated session IDs.

        This method handles the case where session IDs in logs or user input
        may be truncated (e.g., first 8 characters) and searches for the
        full matching session ID.

        Args:
            session_id: Session ID to look up (can be truncated)

        Returns:
            Session data if found, None otherwise
        """
        if not self.jsonl_path.exists():
            logger.warning(f"JSONL file not found: {self.jsonl_path}")
            return None

        try:
            with open(self.jsonl_path, 'r', encoding='utf-8') as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue

                    try:
                        entry = json.loads(line)
                        if entry.get("session_id", "").startswith(session_id):
                            # Found a match - verify it's the correct session
                            full_session_id = entry["session_id"]
                            if full_session_id.startswith(session_id):
                                logger.debug(f"Found session {full_session_id} for lookup {session_id}")
                                return entry
                    except json.JSONDecodeError as e:
                        logger.warning(f"Invalid JSON in session file: {e}")
                        continue

            logger.debug(f"No session found for ID: {session_id}")
            return None

        except IOError as e:
            logger.error(f"Error reading session file: {e}")
            return None

    def file_exists(self, file_path: str) -> bool:
        """
        Check if a file exists, with cross-platform path handling.

        This method provides a wrapper around os.path.exists with better
        error handling and logging for cross-platform compatibility.

        Args:
            file_path: Path to the file to check

        Returns:
            True if file exists, False otherwise
        """
        try:
            exists = os.path.exists(file_path)
            if exists:
                logger.debug(f"File exists: {file_path}")
            else:
                logger.debug(f"File not found: {file_path}")
            return exists
        except Exception as e:
            logger.error(f"Error checking file existence for {file_path}: {e}")
            return False

    def is_daemon_running(self) -> bool:
        """
        Check if a daemon process is already running using PID file.

        This method prevents duplicate daemon spawns by:
        1. Checking if PID file exists
        2. Verifying the PID in the file corresponds to a running process
        3. Handling cases where the process died without cleaning up

        Returns:
            True if daemon is running, False otherwise
        """
        if not self.pid_file or not self.pid_file.exists():
            logger.debug("No PID file found - daemon not running")
            return False

        try:
            # Read PID from file
            pid = int(self.pid_file.read_text().strip())
            logger.debug(f"Found PID file with PID: {pid}")

            # Check if process is running
            try:
                current_pid = os.getpid()

                # Don't consider our own process as a running daemon
                if pid == current_pid:
                    logger.debug("PID file contains current process ID")
                    return False

                # Signal 0 doesn't kill the process, just checks if it exists
                os.kill(pid, 0)
                logger.debug(f"Daemon process {pid} is running")
                return True

            except ProcessLookupError:
                # Process doesn't exist - clean up stale PID file
                logger.debug(f"Process {pid} not found - removing stale PID file")
                try:
                    self.pid_file.unlink()
                except OSError:
                    pass  # File might already be deleted
                return False

            except PermissionError:
                # We don't have permission to signal the process
                # This means it's likely running with different privileges
                logger.debug(f"No permission to signal process {pid} - assuming it's running")
                return True

        except (ValueError, OSError) as e:
            logger.error(f"Error reading PID file {self.pid_file}: {e}")
            # If we can't read the PID file, assume daemon is not running
            return False

    def write_pid_file(self) -> None:
        """
        Write the current process ID to the PID file.

        This is used when starting a daemon process to prevent
        duplicate spawns.
        """
        if not self.pid_file:
            return

        try:
            self.pid_file.parent.mkdir(parents=True, exist_ok=True)
            self.pid_file.write_text(str(os.getpid()))
            logger.debug(f"Written PID {os.getpid()} to {self.pid_file}")
        except OSError as e:
            logger.error(f"Failed to write PID file: {e}")

    def remove_pid_file(self) -> None:
        """
        Remove the PID file.

        This is called when the daemon process shuts down cleanly.
        """
        if not self.pid_file or not self.pid_file.exists():
            return

        try:
            self.pid_file.unlink()
            logger.debug(f"Removed PID file: {self.pid_file}")
        except OSError as e:
            logger.error(f"Failed to remove PID file: {e}")

    def get_all_sessions(self) -> List[Dict[str, Any]]:
        """
        Get all sessions from the JSONL file.

        Returns:
            List of session data entries
        """
        sessions = []

        if not self.jsonl_path.exists():
            logger.warning(f"JSONL file not found: {self.jsonl_path}")
            return sessions

        try:
            with open(self.jsonl_path, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()
                    if not line:
                        continue

                    try:
                        entry = json.loads(line)
                        sessions.append(entry)
                    except json.JSONDecodeError as e:
                        logger.warning(f"Invalid JSON in session file at line {line_num}: {e}")
                        continue

            logger.debug(f"Loaded {len(sessions)} sessions from {self.jsonl_path}")
            return sessions

        except IOError as e:
            logger.error(f"Error reading session file: {e}")
            return sessions

    def add_session(self, session_data: Dict[str, Any]) -> bool:
        """
        Add a new session to the JSONL file.

        Args:
            session_data: Session data to add

        Returns:
            True if successful, False otherwise
        """
        try:
            # Ensure directory exists
            self.jsonl_path.parent.mkdir(parents=True, exist_ok=True)

            # Write session data as JSON line
            with open(self.jsonl_path, 'a', encoding='utf-8') as f:
                f.write(json.dumps(session_data) + '\n')

            logger.debug(f"Added session {session_data.get('session_id')} to {self.jsonl_path}")
            return True

        except IOError as e:
            logger.error(f"Error writing to session file: {e}")
            return False

    def api_request(self, endpoint: str, method: str = 'GET', data: Optional[Dict] = None) -> Optional[Dict]:
        """
        Make an API request to the daemon.

        Args:
            endpoint: API endpoint to call
            method: HTTP method (GET, POST, PUT, DELETE)
            data: Optional data to send with the request

        Returns:
            Response data as dictionary, or None if request failed
        """
        url = urljoin(self.api_base_url, endpoint)

        if not self.requests_available:
            logger.error("requests library not available")
            return None

        try:
            if method.upper() == 'GET':
                response = self.session.get(url, timeout=self.session_timeout)
            elif method.upper() == 'POST':
                response = self.session.post(url, json=data, timeout=self.session_timeout)
            elif method.upper() == 'PUT':
                response = self.session.put(url, json=data, timeout=self.session_timeout)
            elif method.upper() == 'DELETE':
                response = self.session.delete(url, timeout=self.session_timeout)
            else:
                raise ValueError(f"Unsupported HTTP method: {method}")

            response.raise_for_status()
            return response.json() if response.content else {}

        except Exception as e:
            logger.error(f"API request failed: {e}")
            return None

    def get_daemon_status(self) -> Optional[Dict[str, Any]]:
        """
        Get the status of the daemon.

        Returns:
            Status information from the daemon, or None if request failed
        """
        return self.api_request('/status')

    def get_memory_state(self) -> Optional[Dict[str, Any]]:
        """
        Get the current memory state from the daemon.

        Returns:
            Memory state information, or None if request failed
        """
        return self.api_request('/memory/state')

    def set_memory_state(self, state: Dict[str, Any]) -> bool:
        """
        Set the memory state in the daemon.

        Args:
            state: New state to set

        Returns:
            True if successful, False otherwise
        """
        result = self.api_request('/memory/state', method='POST', data=state)
        return result is not None

    def get_shared_state(self, key: str) -> Optional[Any]:
        """
        Get a value from the shared state.

        Args:
            key: Key to retrieve

        Returns:
            Value associated with the key, or None if not found
        """
        response = self.api_request(f'/state/{key}')
        if response and 'value' in response:
            return response['value']
        return None

    def set_shared_state(self, key: str, value: Any) -> bool:
        """
        Set a value in the shared state.

        Args:
            key: Key to set
            value: Value to associate with the key

        Returns:
            True if successful, False otherwise
        """
        data = {'key': key, 'value': value}
        result = self.api_request('/state', method='POST', data=data)
        return result is not None

    def get_cross_terminal_state(self) -> Optional[Dict[str, Any]]:
        """
        Get the cross-terminal state.

        Returns:
            Cross-terminal state information, or None if request failed
        """
        return self.api_request('/cross-terminal/state')

    def register_terminal(self, terminal_id: str, metadata: Optional[Dict[str, Any]] = None) -> bool:
        """
        Register a terminal with the daemon.

        Args:
            terminal_id: Unique identifier for the terminal
            metadata: Optional metadata about the terminal

        Returns:
            True if registration successful, False otherwise
        """
        data = {
            'terminal_id': terminal_id,
            'metadata': metadata or {}
        }
        result = self.api_request('/terminal/register', method='POST', data=data)
        return result is not None

    def heartbeat(self, terminal_id: str) -> bool:
        """
        Send a heartbeat to indicate the terminal is still active.

        Args:
            terminal_id: Unique identifier for the terminal

        Returns:
            True if heartbeat successful, False otherwise
        """
        data = {'terminal_id': terminal_id}
        result = self.api_request('/terminal/heartbeat', method='POST', data=data)
        return result is not None

    def start_daemon(self, daemon_script_path: Optional[str] = None) -> bool:
        """
        Start the memory daemon process.

        Args:
            daemon_script_path: Path to the daemon script (if different from default)

        Returns:
            True if daemon started successfully, False otherwise
        """
        if self.is_daemon_running():
            logger.info("Daemon is already running")
            return True

        try:
            import subprocess
            daemon_path = daemon_script_path or os.path.join(os.path.dirname(__file__), 'daemon_server.py')

            # Start the daemon process
            subprocess.Popen([
                'python', daemon_path,
                '--port', '8080',  # Default port
                '--pid-file', str(self.pid_file) if self.pid_file else '/tmp/maestro_daemon.pid'
            ])

            # Wait a bit for the daemon to start
            time.sleep(2)

            # Check if it's running
            if self.is_daemon_running():
                logger.info("Daemon started successfully")
                return True
            else:
                logger.error("Daemon failed to start properly")
                return False

        except Exception as e:
            logger.error(f"Error starting daemon: {e}")
            return False

    def stop_daemon(self) -> bool:
        """
        Stop the memory daemon process.

        Returns:
            True if daemon stopped successfully, False otherwise
        """
        try:
            result = self.api_request('/shutdown', method='POST')
            if result:
                logger.info("Daemon shutdown request sent successfully")
                # Wait a bit for the daemon to stop
                time.sleep(1)
                return True
            else:
                logger.warning("Daemon shutdown request failed, attempting to kill by PID")
                # If API shutdown failed, try to kill by PID
                if self.pid_file and self.pid_file.exists():
                    try:
                        pid = int(self.pid_file.read_text().strip())
                        os.kill(pid, 15)  # SIGTERM
                        self.pid_file.unlink()  # Remove PID file
                        logger.info(f"Daemon process {pid} terminated")
                        return True
                    except (ValueError, ProcessLookupError, PermissionError) as e:
                        logger.error(f"Error terminating daemon by PID: {e}")
                        return False
                return False
        except Exception as e:
            logger.error(f"Error stopping daemon: {e}")
            return False

    def load_yaml_schema(self) -> Optional[Dict[str, Any]]:
        """
        Load the YAML schema for goal/now/next structures.

        Returns:
            Schema dictionary if loaded successfully, None otherwise
        """
        if not self.yaml_schema_path or not self.yaml_schema_path.exists():
            logger.warning(f"YAML schema file not found: {self.yaml_schema_path}")
            return None

        try:
            with open(self.yaml_schema_path, 'r', encoding='utf-8') as f:
                schema = yaml.safe_load(f)
                logger.debug(f"Loaded YAML schema from {self.yaml_schema_path}")
                return schema
        except Exception as e:
            logger.error(f"Error loading YAML schema: {e}")
            return None

    def validate_goal_now_next_structure(self, data: Dict[str, Any]) -> bool:
        """
        Validate a goal/now/next structure against the schema.

        Args:
            data: Dictionary containing goal, now, next fields

        Returns:
            True if structure is valid, False otherwise
        """
        # Basic validation for required fields
        required_fields = ['goal', 'now', 'next']
        for field in required_fields:
            if field not in data:
                logger.warning(f"Missing required field '{field}' in goal/now/next structure")
                return False

        # Validate that each field is a list or string
        for field in required_fields:
            if not isinstance(data[field], (list, str, type(None))):
                logger.warning(f"Field '{field}' should be a list or string")
                return False

        logger.debug("Goal/now/next structure validation passed")
        return True

    def store_goal_now_next(self, goal_data: Dict[str, Any], session_id: Optional[str] = None) -> bool:
        """
        Store a goal/now/next structure in the daemon.

        Args:
            goal_data: Dictionary containing goal, now, next information
            session_id: Optional session ID to associate with the data

        Returns:
            True if stored successfully, False otherwise
        """
        if not self.validate_goal_now_next_structure(goal_data):
            logger.error("Invalid goal/now/next structure")
            return False

        # Add timestamp and session info if available
        storage_data = {
            'type': 'goal_now_next',
            'timestamp': time.time(),
            'data': goal_data
        }

        if session_id:
            storage_data['session_id'] = session_id

        # Store via API
        result = self.set_shared_state('goal_now_next', storage_data)
        if result:
            logger.info("Goal/now/next data stored successfully")
        else:
            logger.error("Failed to store goal/now/next data")

        return result

    def retrieve_goal_now_next(self, session_id: Optional[str] = None) -> Optional[Dict[str, Any]]:
        """
        Retrieve the most recent goal/now/next structure from the daemon.

        Args:
            session_id: Optional session ID to filter by

        Returns:
            Goal/now/next data if found, None otherwise
        """
        data = self.get_shared_state('goal_now_next')
        if data and isinstance(data, dict):
            # If session_id is specified, check if it matches
            if session_id and data.get('session_id') != session_id:
                return None
            return data.get('data')

        return None

    def update_goal_now_next(self, updates: Dict[str, Any], session_id: Optional[str] = None) -> bool:
        """
        Update specific fields in the goal/now/next structure.

        Args:
            updates: Dictionary with fields to update (subset of goal, now, next)
            session_id: Optional session ID to identify the structure to update

        Returns:
            True if update successful, False otherwise
        """
        current_data = self.retrieve_goal_now_next(session_id)
        if not current_data:
            # If no existing data, create new structure with updates
            current_data = {'goal': None, 'now': None, 'next': None}

        # Update the fields with provided values
        for key, value in updates.items():
            if key in ['goal', 'now', 'next']:
                current_data[key] = value

        return self.store_goal_now_next(current_data, session_id)