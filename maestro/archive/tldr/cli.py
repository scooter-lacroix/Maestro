"""
TLDR CLI Commands for Maestro

Provides command-line interface for TLDR code analysis.
All commands use the /maestro: prefix.
"""

import os
import sys
import click
from typing import Optional, List

from maestro.tldr.analyzer import TLRDAnalyzer
from maestro.tldr.context import get_relevant_context, get_context_for_prompt
from maestro.config.settings import get_settings


@click.group(name="maestro")
def tldr_cli() -> None:
    """Maestro TLDR - Token-efficient code analysis"""
    pass


@tldr_cli.command(name="tree")
@click.argument("path", type=click.Path(exists=True))
@click.option("--ext", "-e", multiple=True, help="File extensions to include")
@click.option("--max-depth", "-d", default=5, help="Maximum directory depth")
@click.option("--show-hidden", is_flag=True, help="Show hidden files")
def tree_command(path: str, ext: tuple, max_depth: int, show_hidden: bool) -> None:
    """
    Show project file tree

    Usage: /maestro:tree [path] [--ext .py] [--max-depth 5]
    """
    from pathlib import Path

    path = os.path.abspath(path)
    extensions = set(ext) if ext else None

    def print_tree(directory: Path, prefix: str = "", depth: int = 0) -> None:
        if depth > max_depth:
            return

        try:
            entries = sorted(directory.iterdir(), key=lambda x: (not x.is_dir(), x.name))
        except PermissionError:
            return

        for i, entry in enumerate(entries):
            # Skip hidden files unless requested
            if not show_hidden and entry.name.startswith("."):
                continue

            # Skip common exclusions
            if entry.name in ("__pycache__", "node_modules", ".git"):
                continue

            is_last = i == len(entries) - 1
            current_prefix = "    " if is_last else "│   "
            connector = "└── " if is_last else "├── "

            # Check extension filter
            if extensions and not entry.is_dir():
                if entry.suffix not in extensions:
                    continue

            click.echo(f"{prefix}{connector}{entry.name}")

            if entry.is_dir():
                new_prefix = prefix + current_prefix
                print_tree(entry, new_prefix, depth + 1)

    click.echo(f"{Path(path).name}/")
    print_tree(Path(path))


@tldr_cli.command(name="structure")
@click.argument("path", type=click.Path(exists=True))
@click.option("--lang", "-l", default="python", help="Language")
@click.option("--max", "-m", default=50, help="Maximum files to analyze")
def structure_command(path: str, lang: str, max: int) -> None:
    """
    Analyze code structure

    Usage: /maestro:structure [path] [--lang python]
    """
    analyzer = TLRDAnalyzer()
    result = analyzer.analyze_file(path)

    if result and result.ast_analysis:
        from maestro.tldr.ast import ASTAnalyzer
        ast_analyzer = ASTAnalyzer()
        click.echo(ast_analyzer.to_llm_string(result.ast_analysis, max_detail=True))
    else:
        click.echo(f"Could not analyze: {path}", err=True)


@tldr_cli.command(name="search")
@click.argument("pattern")
@click.argument("path", type=click.Path(exists=True))
@click.option("--ext", "-e", multiple=True, help="File extensions")
@click.option("-C", context_lines=0, help="Context lines")
@click.option("--max", "-m", default=50, help="Maximum results")
def search_command(pattern: str, path: str, ext: tuple, context_lines: int, max: int) -> None:
    """
    Search code for pattern

    Usage: /maestro:search "pattern" [path]
    """
    import re

    path = os.path.abspath(path)
    extensions = set(ext) if ext else None
    pattern_re = re.compile(pattern, re.IGNORECASE)

    matches = []
    from pathlib import Path

    for py_file in Path(path).rglob("*.py"):
        if extensions and py_file.suffix not in extensions:
            continue

        try:
            with open(py_file, "r", encoding="utf-8") as f:
                for line_num, line in enumerate(f, 1):
                    if pattern_re.search(line):
                        matches.append((str(py_file), line_num, line.rstrip()))
                        if len(matches) >= max:
                            break
        except Exception:
            continue

        if len(matches) >= max:
            break

    for file_path, line_num, line in matches:
        rel_path = os.path.relpath(file_path, path)
        click.echo(f"{rel_path}:{line_num}: {line}")


@tldr_cli.command(name="context")
@click.argument("entry_point")
@click.option("--project", "-p", type=click.Path(exists=True), default=".")
@click.option("--depth", "-d", default=2, help="Call graph depth")
def context_command(entry_point: str, project: str, depth: int) -> None:
    """
    Get context for an entry point

    Usage: /maestro:context <function_or_file> [--project path]
    """
    ctx = get_relevant_context(project, entry_point, depth=depth)

    if ctx:
        click.echo(ctx.to_llm_string())
    else:
        click.echo(f"Could not find context for: {entry_point}", err=True)


@tldr_cli.command(name="cfg")
@click.argument("file_path", type=click.Path(exists=True))
@click.argument("function_name")
def cfg_command(file_path: str, function_name: str) -> None:
    """
    Show control flow graph

    Usage: /maestro:cfg <file> <function>
    """
    analyzer = TLRDAnalyzer()

    try:
        with open(file_path, "r", encoding="utf-8") as f:
            source = f.read()
    except Exception:
        click.echo(f"Could not read file: {file_path}", err=True)
        return

    cfg = analyzer.cfg_analyzer.analyze_function(source, function_name, file_path)

    if cfg:
        click.echo(analyzer.cfg_analyzer.to_llm_string(cfg))
    else:
        click.echo(f"Could not analyze function: {function_name}", err=True)


@tldr_cli.command(name="dfg")
@click.argument("file_path", type=click.Path(exists=True))
@click.argument("function_name")
def dfg_command(file_path: str, function_name: str) -> None:
    """
    Show data flow graph

    Usage: /maestro:dfg <file> <function>
    """
    analyzer = TLRDAnalyzer()

    try:
        with open(file_path, "r", encoding="utf-8") as f:
            source = f.read()
    except Exception:
        click.echo(f"Could not read file: {file_path}", err=True)
        return

    dfg = analyzer.dfg_analyzer.analyze_function(source, function_name, file_path)

    if dfg:
        click.echo(analyzer.dfg_analyzer.to_llm_string(dfg))
    else:
        click.echo(f"Could not analyze function: {function_name}", err=True)


@tldr_cli.command(name="slice")
@click.argument("file_path", type=click.Path(exists=True))
@click.argument("function_name")
@click.argument("line", type=int)
@click.option("--direction", "-d", default="backward", type=click.Choice(["backward", "forward", "both"]))
def slice_command(file_path: str, function_name: str, line: int, direction: str) -> None:
    """
    Perform program slice

    Usage: /maestro:slice <file> <function> <line>
    """
    analyzer = TLRDAnalyzer()

    try:
        with open(file_path, "r", encoding="utf-8") as f:
            source = f.read()
    except Exception:
        click.echo(f"Could not read file: {file_path}", err=True)
        return

    if direction == "backward":
        result = analyzer.slicing_analyzer.slice_backward(source, function_name, line, file_path)
    elif direction == "forward":
        result = analyzer.slicing_analyzer.slice_forward(source, function_name, line, file_path)
    else:
        backward = analyzer.slicing_analyzer.slice_backward(source, function_name, line, file_path)
        forward = analyzer.slicing_analyzer.slice_forward(source, function_name, line, file_path)

        if backward and forward:
            backward.relevant_lines.update(forward.relevant_lines)
            result = backward
        else:
            result = backward or forward

    if result:
        click.echo(f"## Slice: {function_name} @ line {line}")
        click.echo(f"Direction: {direction}")
        click.echo(f"Lines: {len(result.relevant_lines)}")
        click.echo(f"Variables: {', '.join(sorted(result.relevant_variables))}")

        if result.dependencies:
            click.echo("\nDependencies:")
            for dep_line, vars_str in result.dependencies[:20]:
                click.echo(f"  Line {dep_line}: {vars_str}")
    else:
        click.echo(f"Could not slice: {function_name}", err=True)


@tldr_cli.command(name="impact")
@click.argument("function_name")
@click.argument("path", type=click.Path(exists=True))
@click.option("--depth", "-d", default=3, help="Impact depth")
def impact_command(function_name: str, path: str, depth: int) -> None:
    """
    Analyze impact of changing a function

    Usage: /maestro:impact <function> [path]
    """
    analyzer = TLRDAnalyzer()
    result = analyzer.callgraph_analyzer.analyze_impact(function_name, path, depth=depth)

    click.echo(f"## Impact Analysis: {function_name}")
    click.echo(f"Matching locations: {result.get('matching_locations', 0)}")
    click.echo(f"Callers (what calls this): {len(result.get('all_callers', []))}")
    click.echo(f"Callees (what this calls): {len(result.get('all_callees', []))}")

    if result.get("all_callers"):
        click.echo("\n### Callers:")
        for caller in result["all_callers"][:10]:
            click.echo(f"  {caller}")

    if result.get("all_callees"):
        click.echo("\n### Callees:")
        for callee in result["all_callees"][:10]:
            click.echo(f"  {callee}")


@tldr_cli.command(name="daemon")
@click.argument("action", type=click.Choice(["start", "stop", "status", "index"]))
@click.option("--project", "-p", type=click.Path(exists=True), default=".")
@click.option("--port", "-P", default=18766, help="Daemon port")
def daemon_command(action: str, project: str, port: int) -> None:
    """
    Control TLDR daemon for faster queries

    Usage: /maestro:daemon <start|stop|status|index>
    """
    click.echo(f"Daemon {action}: Not yet implemented")
    click.echo(f"Project: {project}")
    click.echo(f"Port: {port}")


@tldr_cli.command(name="index")
@click.argument("path", type=click.Path(exists=True))
@click.option("--force", is_flag=True, help="Force rebuild")
def index_command(path: str, force: bool) -> None:
    """
    Build semantic index

    Usage: /maestro:index [path]
    """
    analyzer = TLRDAnalyzer()
    count = analyzer.build_index(path, force=force)

    click.echo(f"Indexed {count} entities from {path}")


@tldr_cli.command(name="semantic")
@click.argument("query")
@click.option("--project", "-p", type=click.Path(exists=True), default=".")
@click.option("-k", default=10, help="Number of results")
def semantic_command(query: str, project: str, k: int) -> None:
    """
    Semantic code search

    Usage: /maestro:semantic "search query" [--project path]
    """
    analyzer = TLRDAnalyzer()
    results = analyzer.semantic_search(query, project, limit=k)

    click.echo(f"## Semantic Search: {query}")
    click.echo(f"Results: {len(results)}")

    for entity, score in results:
        rel_file = os.path.relpath(entity.file, project)
        type_str = entity.type.upper()
        click.echo(f"\n[{score:.2f}] {type_str}: {entity.name}")
        click.echo(f"  File: {rel_file}:{entity.line}")
        if entity.signature:
            click.echo(f"  {entity.signature}")


def main() -> None:
    """Main CLI entry point"""
    tldr_cli()


if __name__ == "__main__":
    main()
