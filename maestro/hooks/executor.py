"""
Maestro Hook Executor

Provides execution engine for running hooks at various phases.
Integrates with the UnifiedHookManager for coordinated hook execution.
"""

import json
import sys
import os
import subprocess
import shutil
from pathlib import Path
from typing import Optional, Dict, Any, List

# Lazy import of loguru for optional functionality
try:
    from loguru import logger
    LOGURU_AVAILABLE = True
    # Ensure loguru only prints to stderr to avoid polluting stdout (which Claude needs for JSON)
    logger.remove()
    logger.add(sys.stderr, level="INFO")
except ImportError:
    LOGURU_AVAILABLE = False
    # Create a simple fallback logger
    class LoggerStub:
        def debug(self, msg, *args, **kwargs):
            pass
        def info(self, msg, *args, **kwargs):
            print(f"INFO: {msg}", file=sys.stderr)
        def warning(self, msg, *args, **kwargs):
            print(f"WARNING: {msg}", file=sys.stderr)
        def error(self, msg, *args, **kwargs):
            print(f"ERROR: {msg}", file=sys.stderr)
        def critical(self, msg, *args, **kwargs):
            print(f"CRITICAL: {msg}", file=sys.stderr)

    logger = LoggerStub()

# Dependency hygiene: hook execution should work even when the optional memory stack
# (and its deps) is not installed. Fall back to os.getcwd() if unavailable.
try:
    from maestro.memory.utils.detector import detect_project  # type: ignore
except ImportError:  # pragma: no cover
    detect_project = None  # type: ignore[assignment]


def get_python_executable() -> str:
    """
    Get the appropriate Python executable for cross-platform compatibility.

    Priority order:
    1. sys.executable (current environment)
    2. python3 (Unix-like systems)
    3. python (Windows, some Unix systems)

    Returns:
        Path to Python executable
    """
    # Priority 1: Current Python executable (best for venvs)
    exe = sys.executable
    if exe is not None and exe.strip() != "":
        return exe

    # Try python3 next (Unix-like systems)
    exe3 = shutil.which("python3")
    if exe3 is not None:
        return exe3

    # Try python last (Windows, some Unix systems)
    exe_fallback = shutil.which("python")
    if exe_fallback is not None:
        return exe_fallback

    return "python"


class HookExecutor:
    """
    Executes hooks at specified phases.

    Handles hook discovery, execution, and result aggregation.
    """

    def __init__(self, hooks_dir: Optional[Path] = None, include_global_hooks: Optional[bool] = None) -> None:
        """
        Initialize the hook executor.

        Args:
            hooks_dir: Directory containing hooks (default: maestro/hooks)
            include_global_hooks: Whether to include global hooks under
                ~/.claude/plugins/maestro/hooks. If None, defaults to True only when
                using the default local hooks_dir (i.e. hooks_dir is None). This
                keeps test runs hermetic when a custom hooks_dir is supplied.
        """
        # Set up local hooks directory
        use_default_hooks_dir = hooks_dir is None
        if hooks_dir is None:
            maestro_root = Path(__file__).parent.parent
            hooks_dir = maestro_root / "hooks"

        self.hooks_dir = Path(hooks_dir)
        self._hooks_cache: Dict[str, List[Path]] = {}

        # Set up global hooks directory (~/.claude/plugins/maestro/hooks)
        self.global_hooks_dir = Path.home() / ".claude" / "plugins" / "maestro" / "hooks"
        if include_global_hooks is None:
            include_global_hooks = use_default_hooks_dir
        self._include_global_hooks = include_global_hooks

        # Detect project root for CWD
        project_info = detect_project() if callable(detect_project) else None
        self.project_root = project_info.project_path if project_info else os.getcwd()

    def _discover_hooks(self, phase: str) -> List[Path]:
        """
        Discover hooks for a given phase.

        Args:
            phase: Hook phase (e.g., "session-start", "pre-tool-use")

        Returns:
            List of hook file paths (combined local and global)
        """
        if phase in self._hooks_cache:
            return self._hooks_cache[phase]

        hooks = []
        seen_names = set()

        # 1. Discover local hooks
        local_phase_dir = self.hooks_dir / phase
        if local_phase_dir.exists():
            for hook_file in local_phase_dir.glob("*.py"):
                if hook_file.name != "__init__.py":
                    hooks.append(hook_file)
                    seen_names.add(hook_file.name)

        # 2. Discover global hooks (avoiding duplicates)
        if self._include_global_hooks:
            global_phase_dir = self.global_hooks_dir / phase
            if global_phase_dir.exists():
                for hook_file in global_phase_dir.glob("*.py"):
                    if hook_file.name != "__init__.py" and hook_file.name not in seen_names:
                        hooks.append(hook_file)

        self._hooks_cache[phase] = sorted(hooks)
        return self._hooks_cache[phase]

    def execute_hook(
        self,
        phase: str,
        hook_name: str,
        input_data: Dict[str, Any],
    ) -> Dict[str, Any]:
        """
        Execute a single hook.

        Args:
            phase: Hook phase
            hook_name: Name of the hook (without .py extension)
            input_data: Input data for the hook

        Returns:
            Output data from the hook
        """
        # Try local first
        hook_path = self.hooks_dir / phase / f"{hook_name}.py"

        # Fallback to global
        if not hook_path.exists():
            hook_path = self.global_hooks_dir / phase / f"{hook_name}.py"

        if not hook_path.exists():
            logger.warning(f"Hook not found: {phase}/{hook_name}")
            return input_data.copy()

        try:
            # Validate and sanitize input data format
            if not isinstance(input_data, dict):
                input_data = {}
            
            # Run the hook as a subprocess with cross-platform Python executable
            python_exe = get_python_executable()
            if not python_exe:
                python_exe = "python"
                
            result = subprocess.run(
                [python_exe, str(hook_path)],
                input=json.dumps(input_data),
                capture_output=True,
                text=True,
                timeout=30,
                cwd=self.project_root,  # Ensure execution from project root
            )

            if result.returncode != 0:
                logger.error(f"Hook {phase}/{hook_name} failed: {result.stderr}")
                input_data["hook_error"] = result.stderr.strip()
                return input_data

            # Parse output
            output = json.loads(result.stdout) if result.stdout.strip() else input_data
            return output

        except subprocess.TimeoutExpired:
            logger.error(f"Hook {phase}/{hook_name} timed out")
            input_data["hook_error"] = "Timeout"
            return input_data
        except FileNotFoundError:
            logger.error(f"Python executable not found for hook {phase}/{hook_name}")
            input_data["hook_error"] = "Python Executable Missing"
            return input_data
        except json.JSONDecodeError as e:
            logger.error(f"Hook {phase}/{hook_name} returned invalid JSON: {e}")
            input_data["hook_error"] = f"Invalid JSON: {e}"
            return input_data
        except Exception as e:
            logger.error(f"Hook {phase}/{hook_name} error: {e}")
            input_data["hook_error"] = str(e)
            return input_data

    def execute_phase(
        self,
        phase: str,
        input_data: Dict[str, Any],
    ) -> Dict[str, Any]:
        """
        Execute all hooks for a phase.

        Args:
            phase: Hook phase
            input_data: Input data for hooks

        Returns:
            Output data after all hooks
        """
        hooks = self._discover_hooks(phase)

        if not hooks and phase != "loop":
            return input_data

        output_data = input_data.copy()
        phase_results = []

        for hook_path in hooks:
            hook_name = hook_path.stem
            result = self.execute_hook(phase, hook_name, output_data)
            # Merge result into output_data to preserve state even if the hook returns a fresh dict
            if isinstance(result, dict):
                output_data.update(result)
            else:
                logger.warning(
                    "Hook '%s' returned non-dict type %s; skipping merge to preserve pipeline state",
                    hook_name,
                    type(result).__name__,
                )

            phase_results.append({
                "hook": hook_name,
                "success": "hook_error" not in result,
            })

        nexus_result = self._emit_nexus_lifecycle(phase, output_data)
        if nexus_result is not None:
            output_data.setdefault("nexus_lifecycle", []).append(nexus_result)

        # Some hook outcomes represent higher-level cognition transitions rather than
        # the raw phase itself. Emit those as explicit Nexus lifecycle events so the
        # subconscious runtime sees review/checkpoint boundaries in addition to stop/end.
        if output_data.get("review_point_reached"):
            review_result = self._emit_nexus_lifecycle("review", output_data)
            if review_result is not None:
                output_data.setdefault("nexus_lifecycle", []).append(review_result)
        if output_data.get("checkpoint_reached"):
            checkpoint_result = self._emit_nexus_lifecycle("checkpoint", output_data)
            if checkpoint_result is not None:
                output_data.setdefault("nexus_lifecycle", []).append(checkpoint_result)

        output_data[f"{phase}_results"] = phase_results

        return output_data

    def _emit_nexus_lifecycle(
        self,
        phase: str,
        input_data: Dict[str, Any],
    ) -> Optional[Dict[str, Any]]:
        """Best-effort transport shim from Maestro hook phases to Nexus session lifecycle."""
        nexus_bin = os.environ.get("NEXUS_BIN") or shutil.which("nexus")
        if not nexus_bin:
            return None

        session_id = str(
            input_data.get("session_id")
            or input_data.get("session_key")
            or os.environ.get("MAESTRO_SESSION_ID", "")
        ).strip()
        project_path = str(
            input_data.get("project_path")
            or input_data.get("cwd")
            or os.environ.get("MAESTRO_PROJECT_PATH", "")
        ).strip()
        agent = str(
            input_data.get("tool")
            or input_data.get("agent_tool")
            or input_data.get("selected_cli")
            or os.environ.get("MAESTRO_SELECTED_CLI", "claude-code")
        ).strip()

        if not session_id or not project_path:
            return None

        phase_to_command = {
            "session-start": ["session", "start", "--agent", agent],
            "pre-compact": ["session", "event", "--agent", agent, "--kind", "compact"],
            "loop": ["session", "event", "--agent", agent, "--kind", "loop"],
            "subagent-stop": ["session", "event", "--agent", agent, "--kind", "completion"],
            "session-end": ["session", "end", "--agent", agent, "--reason", "session-end"],
            "review": ["session", "event", "--agent", agent, "--kind", "review"],
            "checkpoint": ["session", "event", "--agent", agent, "--kind", "checkpoint"],
        }

        args = phase_to_command.get(phase)
        if args is None:
            return None

        args = args + ["--session-key", session_id, "--cwd", project_path]
        env = os.environ.copy()
        env["MAESTRO_NEXUS_EVENT_PAYLOAD"] = json.dumps({
            "managed_session": True,
            "provider_profile": os.environ.get("MAESTRO_PROVIDER_PROFILE", "maestro_runtime"),
            "session_id": session_id,
            "project_root": project_path,
            "track_id": input_data.get("track_id"),
            "current_task_id": input_data.get("current_task_id") or input_data.get("task_id"),
            "launch_origin": input_data.get("launch_origin"),
            "selected_cli": agent,
            "event_reason": phase,
        })

        try:
            result = subprocess.run(
                [nexus_bin, *args],
                capture_output=True,
                text=True,
                timeout=20,
                cwd=self.project_root,
                env=env,
            )
            return {
                "phase": phase,
                "command": [nexus_bin, *args],
                "success": result.returncode == 0,
                "stderr": result.stderr.strip(),
                "stdout": result.stdout.strip(),
            }
        except subprocess.TimeoutExpired:
            return {
                "phase": phase,
                "command": [nexus_bin, *args],
                "success": False,
                "stderr": "Nexus connection or execution timed out.",
                "stdout": "",
            }
        except FileNotFoundError:
            return {
                "phase": phase,
                "command": [nexus_bin, *args],
                "success": False,
                "stderr": "Nexus executable not found.",
                "stdout": "",
            }
        except Exception as exc:
            return {
                "phase": phase,
                "command": [nexus_bin, *args],
                "success": False,
                "stderr": f"Network or execution failure: {str(exc)}",
                "stdout": "",
            }

    def execute_chain(
        self,
        phases: List[str],
        input_data: Dict[str, Any],
    ) -> Dict[str, Any]:
        """
        Execute hooks across multiple phases in sequence.

        Args:
            phases: List of phases to execute
            input_data: Input data for hooks

        Returns:
            Output data after all phases
        """
        output_data = input_data.copy()

        for phase in phases:
            output_data = self.execute_phase(phase, output_data)

        return output_data


# Global executor instance
_global_executor: Optional[HookExecutor] = None


def get_hook_executor() -> HookExecutor:
    """Get the global hook executor instance."""
    global _global_executor

    if _global_executor is None:
        _global_executor = HookExecutor()

    return _global_executor


def execute_session_start(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all session-start hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("session-start", input_data)


def execute_pre_tool_use(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all pre-tool-use hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("pre-tool-use", input_data)


def execute_post_tool_use(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all post-tool-use hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("post-tool-use", input_data)


def execute_pre_compact(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all pre-compact hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("pre-compact", input_data)


def execute_user_prompt_submit(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all user-prompt-submit hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("user-prompt-submit", input_data)


def execute_subagent_stop(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all subagent-stop hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("subagent-stop", input_data)


def execute_review(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all review hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("review", input_data)


def execute_checkpoint(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all checkpoint hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("checkpoint", input_data)


def execute_session_end(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all session-end hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("session-end", input_data)
