/**
 * Template generation for spec.md and plan.md
 *
 * Provides functions to generate well-formatted maestro documents
 * following the standard maestro file format.
 */

export interface SpecOptions {
  title: string;
  description: string;
  trackId: string;
  type: "feature" | "bug" | "chore" | "refactor" | "master";
  requirements: {
    users?: string;
    goals?: string;
    constraints?: string;
    acceptanceCriteria?: string[];
  };
}

export interface PlanOptions {
  title: string;
  spec: string;
  phases: Phase[];
}

export interface Phase {
  name: string;
  tasks: Task[];
}

export interface Task {
  description: string;
  subtasks?: string[];
}

/** Generate spec.md content */
export function generateSpec(options: SpecOptions): string {
  const { title, description, trackId, type, requirements } = options;
  const now = new Date().toISOString().slice(0, 10);

  return `# Specification: ${title}

**Track ID:** ${trackId}
**Type:** ${type}
**Status:** new
**Created:** ${now}

## Overview

${description}

## Vision Statement

${requirements.goals || "To be defined during planning."}

## Goals

### Primary Goals

${requirements.goals ? requirements.goals.split("\n").map(g => `- ${g}`).join("\n") : "- To be defined during planning."}

### Non-Goals

- Features that are explicitly out of scope for this track

## Requirements

### Functional Requirements

#### FR1: Core Functionality
- FR1.1: ${description}

### Non-Functional Requirements

#### NFR1: Performance
- Response time requirements

#### NFR2: Reliability
- Error handling requirements

#### NFR3: Maintainability
- Code quality and documentation standards

## Acceptance Criteria

${requirements.acceptanceCriteria ? requirements.acceptanceCriteria.map((c, i) => `${i + 1}. [ ] ${c}`).join("\n") : "1. [ ] Criteria 1\n2. [ ] Criteria 2\n3. [ ] Criteria 3"}

## Architecture

### System Design

- High-level architecture to be defined during planning

### Components

- Component breakdown to be defined during planning

## Dependencies

### External Dependencies
- List external service dependencies

### Internal Dependencies
- List internal module dependencies

## Success Criteria

### Metrics
- [ ] Measurable success criteria
- [ ] Performance benchmarks
- [ ] User satisfaction targets

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Risk 1 | High | Mitigation strategy |
| Risk 2 | Medium | Mitigation strategy |

## Out of Scope

- Features explicitly not included in this track
- Future enhancements to be considered separately
`;
}

/** Generate plan.md content */
export function generatePlan(options: PlanOptions): string {
  const { title, phases } = options;

  let content = `# Implementation Plan: ${title}

This plan outlines the phased approach to implementing the specification.

`;

  for (let i = 0; i < phases.length; i++) {
    const phase = phases[i];
    content += `## Phase ${i + 1}: ${phase.name}

`;

    for (const task of phase.tasks) {
      content += `- [ ] Task: ${task.description}
`;
      if (task.subtasks && task.subtasks.length > 0) {
        for (const subtask of task.subtasks) {
          content += `  - [ ] ${subtask}
`;
        }
      }
      content += "\n";
    }
  }

  return content.trimEnd();
}

/** Generate default phases for a feature track */
export function generateDefaultPhases(): Phase[] {
  return [
    {
      name: "Analysis & Design",
      tasks: [
        { description: "Analyze requirements and constraints", subtasks: ["Review specification", "Identify edge cases"] },
        { description: "Design solution approach", subtasks: ["Architecture decisions", "Technology choices"] },
      ],
    },
    {
      name: "Implementation",
      tasks: [
        { description: "Implement core functionality", subtasks: ["Set up project structure", "Write core logic"] },
        { description: "Add error handling and validation", subtasks: ["Input validation", "Error messages"] },
        { description: "Write tests", subtasks: ["Unit tests", "Integration tests"] },
      ],
    },
    {
      name: "Documentation & Review",
      tasks: [
        { description: "Document changes", subtasks: ["Update README", "Add code comments"] },
        { description: "Code review and refinement", subtasks: ["Self-review", "Address feedback"] },
      ],
    },
  ];
}

/** Template for asking user questions during newTrack */
export function generateQuestionPrompt(question: string, options: string[]): string {
  return `# Question: ${question}

${options.map((opt, i) => `${i + 1}. ${opt}`).join("\n")}

Please select an option or type your own answer.
`;
}
