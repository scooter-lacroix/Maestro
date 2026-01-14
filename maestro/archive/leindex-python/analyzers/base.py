from abc import ABC, abstractmethod
from typing import Dict, Any, Optional

class BaseAnalyzer(ABC):
    """Abstract base class for all code analyzers."""

    @abstractmethod
    def analyze(self, code: str, file_path: str) -> Dict[str, Any]:
        """
        Analyze the given code.

        Args:
            code: The source code to analyze.
            file_path: The path to the file being analyzed.

        Returns:
            A dictionary containing the analysis results.
        """
        pass

    @abstractmethod
    def to_llm_string(self, analysis_result: Dict[str, Any]) -> str:
        """
        Convert analysis results to a token-efficient string representation for LLMs.

        Args:
            analysis_result: The result dictionary from `analyze`.

        Returns:
            A string representation optimized for LLM consumption.
        """
        pass
