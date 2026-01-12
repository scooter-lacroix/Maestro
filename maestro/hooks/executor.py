"""
Maestro Hook Executor

Provides execution engine for running hooks at various phases.
Integrates with the UnifiedHookManager for coordinated hook execution.
"""

import json
import sys
import os
import subprocess
from pathlib import Path
from typing import Optional, Dict, Any, List
from loguru import logger


class HookExecutor:
    """
    Executes hooks at specified phases.

    Handles hook discovery, execution, and result aggregation.
    """

    def __init__(self, hooks_dir: Optional[Path] = None) -> None:
        """
        Initialize the hook executor.

        Args:
            hooks_dir: Directory containing hooks (default: maestro/hooks)
        """
        if hooks_dir is None:
            maestro_root = Path(__file__).parent.parent
            hooks_dir = maestro_root / "hooks"

        self.hooks_dir = Path(hooks_dir)
        self._hooks_cache: Dict[str, List[Path]] = {}

    def _discover_hooks(self, phase: str) -> List[Path]:
        """
        Discover hooks for a given phase.

        Args:
            phase: Hook phase (e.g., "session-start", "pre-tool-use")

        Returns:
            List of hook file paths
        """
        if phase in self._hooks_cache:
            return self._hooks_cache[phase]

        phase_dir = self.hooks_dir / phase

        if not phase_dir.exists():
            self._hooks_cache[phase] = []
            return []

        hooks = []
        for hook_file in phase_dir.glob("*.py"):
            if hook_file.name != "__init__.py":
                hooks.append(hook_file)

        self._hooks_cache[phase] = sorted(hooks)
        return hooks

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
        hook_path = self.hooks_dir / phase / f"{hook_name}.py"

        if not hook_path.exists():
            logger.warning(f"Hook not found: {phase}/{hook_name}")
            return input_data.copy()

        try:
            # Run the hook as a subprocess
            result = subprocess.run(
                [sys.executable, str(hook_path)],
                input=json.dumps(input_data),
                capture_output=True,
                text=True,
                timeout=30,
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

        if not hooks:
            return input_data

        output_data = input_data.copy()
        phase_results = []

        for hook_path in hooks:
            hook_name = hook_path.stem
            result = self.execute_hook(phase, hook_name, output_data)
            output_data = result

            phase_results.append({
                "hook": hook_name,
                "success": "hook_error" not in result,
            })

        output_data[f"{phase}_results"] = phase_results

        return output_data

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


def execute_session_end(input_data: Dict[str, Any]) -> Dict[str, Any]:
    """Execute all session-end hooks."""
    executor = get_hook_executor()
    return executor.execute_phase("session-end", input_data)
