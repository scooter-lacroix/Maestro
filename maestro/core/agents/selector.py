"""
Maestro Agent Selector

Implements complexity-based agent selection for the Maestro v2 framework.
Integrates with the agent registry to recommend appropriate agents based on task characteristics.
Supports built-in-first selection with optional external reviewer fallback.
"""

from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, List, Dict, Any
import shutil
import yaml  # type: ignore


@dataclass
class AgentDefinition:
    """Represents an agent in the registry."""
    name: str
    file: str
    model: str
    description: str
    tools: List[str]
    complexity_threshold: str
    best_for: List[str]
    category: str
    output_path: Optional[str] = None
    supports_checkpointing: bool = False
    fallback_for: List[str] = field(default_factory=list)
    requires_visual_evidence: bool = False
    external_backstops: List[str] = field(default_factory=list)
    supports_graceful_fallback: bool = False


@dataclass
class TaskContext:
    """Context for selecting an appropriate agent."""
    task_type: str  # implement, refactor, plan, debug, etc.
    complexity: str  # trivial, small, medium, large, very_large
    file_count: int = 1
    phase_count: int = 1
    keywords: List[str] = field(default_factory=list)
    constraints: List[str] = field(default_factory=list)


class AgentRegistry:
    """
    Manages the agent registry and provides lookup functionality.
    """

    def __init__(self, registry_path: Optional[Path] = None):
        """
        Initialize the registry from a YAML file.

        Args:
            registry_path: Path to registry.yaml. Defaults to maestro/agents/registry.yaml
        """
        if registry_path is None:
            # Default to package-relative path
            registry_path = Path(__file__).parent.parent.parent / "agents" / "registry.yaml"

        self.registry_path = registry_path
        self.agents_by_name: Dict[str, AgentDefinition] = {}
        self.agents_by_category: Dict[str, List[AgentDefinition]] = {}
        self.agents_by_complexity: Dict[str, List[AgentDefinition]] = {}
        self.keyword_mapping: Dict[str, List[str]] = {}
        self.complexity_thresholds: Dict[str, Dict[str, Any]] = {}

        self._load_registry()

    def _load_registry(self) -> None:
        """Load the registry from YAML file."""
        if not self.registry_path.exists():
            raise FileNotFoundError(f"Registry file not found: {self.registry_path}")

        with open(self.registry_path, "r", encoding="utf-8") as f:
            data = yaml.safe_load(f)

        # Load agents by category
        for category_name, category_data in data.get("categories", {}).items():
            category_agents = []
            for agent_data in category_data.get("agents", []):
                agent = AgentDefinition(
                    name=agent_data["name"],
                    file=agent_data["file"],
                    model=agent_data["model"],
                    description=agent_data["description"],
                    tools=agent_data.get("tools", []),
                    complexity_threshold=agent_data.get("complexity_threshold", "medium"),
                    best_for=agent_data.get("best_for", []),
                    category=category_name,
                    output_path=agent_data.get("output_path"),
                    supports_checkpointing=agent_data.get("supports_checkpointing", False),
                    fallback_for=agent_data.get("fallback_for", []),
                    requires_visual_evidence=agent_data.get("requires_visual_evidence", False),
                    external_backstops=agent_data.get("external_backstops", []),
                    supports_graceful_fallback=agent_data.get("supports_graceful_fallback", False),
                )
                self.agents_by_name[agent.name] = agent
                category_agents.append(agent)

            self.agents_by_category[category_name] = category_agents

        # Load complexity thresholds
        for level, level_data in data.get("complexity", {}).items():
            self.complexity_thresholds[level] = level_data
            # Map suggested agents to this complexity level
            for agent_name in level_data.get("suggested_agents", []):
                if agent_name in self.agents_by_name:
                    self.agents_by_complexity.setdefault(level, []).append(
                        self.agents_by_name[agent_name]
                    )

        # Load keyword mapping
        self.keyword_mapping = data.get("selection", {}).get("keyword_mapping", {})

    def get_agent(self, name: str) -> Optional[AgentDefinition]:
        """Get an agent by name."""
        return self.agents_by_name.get(name)

    def get_agents_by_category(self, category: str) -> List[AgentDefinition]:
        """Get all agents in a category."""
        return self.agents_by_category.get(category, [])

    def get_agents_by_complexity(self, complexity: str) -> List[AgentDefinition]:
        """Get all agents suitable for a complexity level."""
        return self.agents_by_complexity.get(complexity, [])

    def get_all_agents(self) -> List[AgentDefinition]:
        """Get all registered agents."""
        return list(self.agents_by_name.values())


class AgentSelector:
    """
    Selects appropriate agents based on task context and complexity.
    """

    def __init__(self, registry: Optional[AgentRegistry] = None):
        """
        Initialize the selector with an agent registry.

        Args:
            registry: AgentRegistry instance. Creates a new one if not provided.
        """
        self.registry = registry or AgentRegistry()

    def select_agent(self, context: TaskContext) -> Optional[AgentDefinition]:
        """
        Select the most appropriate agent for a given task context.

        Args:
            context: Task context including type, complexity, and constraints

        Returns:
            The selected AgentDefinition, or None if no match found
        """
        # First, try keyword-based selection
        if context.task_type in self.registry.keyword_mapping:
            suggested_names = self.registry.keyword_mapping[context.task_type]
            # Filter by complexity
            for name in suggested_names:
                agent = self.registry.get_agent(name)
                if agent and self._matches_complexity(agent, context.complexity):
                    return agent

        # Fall back to complexity-based selection
        suitable_agents = self.registry.get_agents_by_complexity(context.complexity)
        if suitable_agents:
            # Return the first match, or prefer opus models for complex tasks
            if context.complexity in ("medium", "large", "very_large"):
                opus_agents = [a for a in suitable_agents if a.model == "opus"]
                if opus_agents:
                    return opus_agents[0]
            return suitable_agents[0]

        # Final fallback - return any agent that matches task type
        if context.task_type in self.registry.keyword_mapping:
            suggested_names = self.registry.keyword_mapping[context.task_type]
            for name in suggested_names:
                agent = self.registry.get_agent(name)
                if agent:
                    return agent

        return None

    def select_agents_for_workflow(self, phases: List[Dict[str, Any]]) -> List[AgentDefinition]:
        """
        Select multiple agents for a multi-phase workflow.

        Args:
            phases: List of phase dictionaries with 'task_type' and 'complexity'

        Returns:
            List of AgentDefinition objects for each phase
        """
        selected = []
        for phase in phases:
            context = TaskContext(
                task_type=phase.get("task_type", "implement"),
                complexity=phase.get("complexity", "medium"),
                file_count=phase.get("file_count", 1),
                phase_count=phase.get("phase_count", 1),
                keywords=phase.get("keywords", []),
            )
            agent = self.select_agent(context)
            if agent:
                selected.append(agent)
        return selected

    def _matches_complexity(self, agent: AgentDefinition, complexity: str) -> bool:
        """Check if an agent's complexity threshold matches the required complexity."""
        agent_complexity = agent.complexity_threshold

        # Define complexity hierarchy
        hierarchy = {
            "trivial": 0,
            "small": 1,
            "medium": 2,
            "large": 3,
            "very_large": 4
        }

        required = hierarchy.get(complexity, 2)
        agent_level = hierarchy.get(agent_complexity, 2)

        # Agent can handle tasks at or below its level
        return agent_level >= required

    def select_with_fallback(
        self,
        context: TaskContext,
        external_cli_check: bool = True,
    ) -> Dict[str, Any]:
        """
        Select agents using built-in-first, external-second strategy.

        Returns a dict with the primary built-in agent, and optionally
        an external final-pass reviewer if its CLI is available.

        Args:
            context: Task context for selection
            external_cli_check: Whether to probe for external CLI availability

        Returns:
            Dict with 'primary' (built-in agent), 'final_reviewer' (optional external),
            and 'fallback_used' (bool indicating if built-in replaced external)
        """
        primary = self.select_agent(context)
        result: Dict[str, Any] = {
            "primary": primary,
            "final_reviewer": None,
            "fallback_used": False,
            "message": None,
        }

        if primary is None:
            return result

        # Find a built-in final reviewer (warden) for the review stage
        warden = self.registry.get_agent("warden")

        # Check if any external backstop is available
        external_available = None
        if external_cli_check and primary.external_backstops:
            for cli_name in primary.external_backstops:
                if is_external_cli_available(cli_name):
                    external_available = cli_name
                    break

        if external_available:
            result["final_reviewer"] = external_available
            result["message"] = f"External final-pass reviewer available: {external_available}"
        elif warden and warden.name != primary.name:
            result["final_reviewer"] = warden
            result["fallback_used"] = True
            result["message"] = (
                "Using built-in final reviewer (warden). "
                "No external CLI reviewer detected."
            )

        return result

    def recommend_orchestration_pattern(
        self,
        context: TaskContext
    ) -> Dict[str, Any]:
        """
        Recommend an orchestration pattern for complex tasks.

        Args:
            context: Task context

        Returns:
            Dictionary with pattern recommendation and agent assignments
        """
        if context.complexity in ("large", "very_large"):
            return {
                "pattern": "hierarchical",
                "orchestrator": "maestro" if context.complexity == "large" else "orchestrate-large",
                "recommended_workflow": self._hierarchical_workflow(context)
            }

        if context.phase_count > 1:
            return {
                "pattern": "pipeline",
                "recommended_workflow": self._pipeline_workflow(context)
            }

        return {
            "pattern": "single_agent",
            "recommended_agent": self.select_agent(context)
        }

    def _hierarchical_workflow(self, context: TaskContext) -> List[Dict[str, Any]]:
        """Define a hierarchical workflow for complex tasks."""
        workflow = [
            {"phase": "research", "task_type": "explore", "complexity": "small"},
            {"phase": "planning", "task_type": "plan", "complexity": "medium"},
            {"phase": "implementation", "task_type": "implement", "complexity": "medium"},
            {"phase": "validation", "task_type": "test", "complexity": "small"},
        ]
        return workflow

    def _pipeline_workflow(self, context: TaskContext) -> List[Dict[str, Any]]:
        """Define a pipeline workflow for sequential tasks."""
        return [
            {"phase": "analysis", "task_type": context.task_type, "complexity": context.complexity}
        ]


def estimate_complexity(
    file_count: int,
    phase_count: int = 1,
    has_external_dependencies: bool = False,
    is_migration: bool = False
) -> str:
    """
    Estimate task complexity based on characteristics.

    Args:
        file_count: Number of files to modify
        phase_count: Number of implementation phases
        has_external_dependencies: Whether task depends on external systems
        is_migration: Whether task is a migration

    Returns:
        Complexity level: trivial, small, medium, large, or very_large
    """
    if file_count <= 1 and phase_count == 1:
        return "trivial"

    if file_count <= 5 and phase_count <= 2:
        return "small"

    if file_count <= 15 and phase_count <= 4:
        if has_external_dependencies or is_migration:
            return "large"
        return "medium"

    if file_count <= 30:
        return "large"

    return "very_large"


# Convenience functions for common use cases
def quick_select(task_type: str, complexity: str = "medium") -> Optional[str]:
    """
    Quick agent selection by task type and complexity.

    Args:
        task_type: Type of task (implement, refactor, plan, etc.)
        complexity: Complexity level (trivial, small, medium, large, very_large)

    Returns:
        Name of the selected agent, or None if no match
    """
    registry = AgentRegistry()
    selector = AgentSelector(registry)
    context = TaskContext(task_type=task_type, complexity=complexity)
    agent = selector.select_agent(context)
    return agent.name if agent else None


def list_agents(category: Optional[str] = None) -> List[str]:
    """
    List available agents, optionally filtered by category.

    Args:
        category: Category to filter by (orchestrators, planners, etc.)

    Returns:
        List of agent names
    """
    registry = AgentRegistry()
    if category:
        agents = registry.get_agents_by_category(category)
    else:
        agents = registry.get_all_agents()
    return [agent.name for agent in agents]


def is_external_cli_available(cli_name: str) -> bool:
    """
    Check whether an external CLI tool is available on the system PATH.

    Args:
        cli_name: Name of the CLI executable (e.g., 'codex-cli', 'gemini-cli')

    Returns:
        True if the CLI is found on PATH, False otherwise
    """
    return shutil.which(cli_name) is not None


def get_available_external_reviewers(backstop_list: Optional[List[str]] = None) -> List[str]:
    """
    Return the subset of external CLI reviewers that are currently available.

    Args:
        backstop_list: List of CLI names to check. If None, checks all known reviewers.

    Returns:
        List of available CLI names
    """
    known_reviewers = backstop_list or [
        "codex-cli", "gemini-cli", "qwen-cli", "opencode"
    ]
    return [cli for cli in known_reviewers if is_external_cli_available(cli)]
