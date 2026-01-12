"""
Maestro TLDR - Token-efficient code analysis

Provides 5-layer code analysis for dramatic token savings:
- Layer 1: AST (Abstract Syntax Tree)
- Layer 2: Call Graph
- Layer 3: CFG (Control Flow Graph)
- Layer 4: DFG (Data Flow Graph)
- Layer 5: Slicing (Program Slicing)
"""

from maestro.tldr.version import __version__
from maestro.tldr.ast import ASTAnalyzer
from maestro.tldr.callgraph import CallGraphAnalyzer
from maestro.tldr.cfg import CFGAnalyzer
from maestro.tldr.dfg import DFGAnalyzer
from maestro.tldr.slicing import SlicingAnalyzer
from maestro.tldr.semantic import SemanticIndex
from maestro.tldr.context import get_relevant_context
from maestro.tldr.analyzer import TLRDAnalyzer

__all__ = [
    "__version__",
    "ASTAnalyzer",
    "CallGraphAnalyzer",
    "CFGAnalyzer",
    "DFGAnalyzer",
    "SlicingAnalyzer",
    "SemanticIndex",
    "get_relevant_context",
    "TLRDAnalyzer",
]
