/**
 * Critical Think integration
 *
 * Provides functions to load and apply Critical Think templates
 * for metacognitive analysis at key integration points.
 *
 * Integration points:
 * - Before questions (newTrack Q&A phase)
 * - After answers (validation)
 * - Before documentation (spec/plan generation)
 * - After documentation (quality check)
 * - Before implementation (plan analysis)
 * - After implementation (validation)
 * - Before agent delegation (prevent over-delegation)
 * - After agent delegation (validate work)
 */

import * as fs from "fs";
import * as path from "path";

/** Critical Think template types */
export type CriticalThinkTemplate =
  | "question"
  | "after_action"
  | "before_action"
  | "docs"
  | "implementation"
  | "agent_delegation";

/** Template directory paths */
let templatesDir: string | null = null;

/**
 * Initialize Critical Think templates directory
 * Copies templates from package to project if needed
 */
export function initCriticalThinkTemplates(root: string, packageTemplatesDir: string): void {
  const projectTemplatesDir = path.join(root, "maestro/critical_think/templates");
  fs.mkdirSync(projectTemplatesDir, { recursive: true });

  // Copy templates from package to project
  const templateFiles: CriticalThinkTemplate[] = [
    "question",
    "after_action",
    "before_action",
    "docs",
    "implementation",
    "agent_delegation",
  ];

  for (const template of templateFiles) {
    const srcPath = path.join(packageTemplatesDir, `criticalthink_${template}.md`);
    const destPath = path.join(projectTemplatesDir, `criticalthink_${template}.md`);

    if (fs.existsSync(srcPath) && !fs.existsSync(destPath)) {
      fs.copyFileSync(srcPath, destPath);
    }
  }

  templatesDir = projectTemplatesDir;
}

/**
 * Load a Critical Think template
 */
export function loadCriticalThinkTemplate(template: CriticalThinkTemplate): string {
  if (!templatesDir) {
    return ""; // Templates not initialized
  }

  const templatePath = path.join(templatesDir, `criticalthink_${template}.md`);

  if (!fs.existsSync(templatePath)) {
    return ""; // Template not found
  }

  return fs.readFileSync(templatePath, "utf-8");
}

/**
 * Apply Critical Think analysis for a question
 *
 * Returns a prompt that applies the 6-step Critical Think framework
 */
export function applyCriticalThinkForQuestion(question: string, context: {
  currentStep: string;
  confidence?: number;
}): string {
  const template = loadCriticalThinkTemplate("question");

  if (!template) {
    return ""; // No template available
  }

  return template
    .replace(/{{QUESTION}}/g, question)
    .replace(/{{CONTEXT}}/g, context.currentStep)
    .replace(/{{CONFIDENCE}}/g, String(context.confidence || 5));
}

/**
 * Apply Critical Think analysis before documentation generation
 */
export function applyCriticalThinkForDocs(documentType: "spec" | "plan", context: string): string {
  const template = loadCriticalThinkTemplate("docs");

  if (!template) {
    return "";
  }

  return template
    .replace(/{{DOC_TYPE}}/g, documentType)
    .replace(/{{CONTEXT}}/g, context);
}

/**
 * Apply Critical Think analysis before implementation
 */
export function applyCriticalThinkForImplementation(taskDescription: string, planContext: string): string {
  const template = loadCriticalThinkTemplate("implementation");

  if (!template) {
    return "";
  }

  return template
    .replace(/{{TASK}}/g, taskDescription)
    .replace(/{{CONTEXT}}/g, planContext);
}

/**
 * Apply Critical Think analysis before agent delegation
 */
export function applyCriticalThinkForAgentDelegation(
  taskDescription: string,
  agentType: string,
  complexity: "trivial" | "standard" | "complex"
): string {
  const template = loadCriticalThinkTemplate("agent_delegation");

  if (!template) {
    return "";
  }

  return template
    .replace(/{{TASK}}/g, taskDescription)
    .replace(/{{AGENT_TYPE}}/g, agentType)
    .replace(/{{COMPLEXITY}}/g, complexity);
}

/**
 * Apply Critical Think analysis after any action
 */
export function applyCriticalThinkAfterAction(
  action: string,
  result: string,
  expectedOutcome: string
): string {
  const template = loadCriticalThinkTemplate("after_action");

  if (!template) {
    return "";
  }

  return template
    .replace(/{{ACTION}}/g, action)
    .replace(/{{RESULT}}/g, result)
    .replace(/{{EXPECTED}}/g, expectedOutcome);
}

/**
 * Apply Critical Think analysis before any action
 */
export function applyCriticalThinkBeforeAction(action: string, context: string): string {
  const template = loadCriticalThinkTemplate("before_action");

  if (!template) {
    return "";
  }

  return template
    .replace(/{{ACTION}}/g, action)
    .replace(/{{CONTEXT}}/g, context);
}

/**
 * Check if Critical Think should be applied for a question
 * (based on confidence level and necessity)
 */
export function shouldApplyCriticalThinkForQuestion(confidence: number, isNecessary: boolean): boolean {
  // Apply Critical Think if:
  // 1. Confidence is low (< 7/10)
  // 2. Question is not truly necessary
  return confidence < 7 || !isNecessary;
}

/**
 * Format Critical Think analysis result for display
 */
export function formatCriticalThinkPrompt(analysis: {
  step1: string;
  step2: string;
  step3: string;
  step4: string;
  step5: string;
  decision: "PROCEED" | "SKIP" | "REFINE";
}): string {
  return `
# Critical Think Analysis

## Step 1: Core Thesis & Confidence
${analysis.step1}

## Step 2: Foundational Analysis
${analysis.step2}

## Step 3: Logical Integrity Check
${analysis.step3}

## Step 4: AI-Specific Pitfall Check
${analysis.step4}

## Step 5: Risk & Mitigation
${analysis.step5}

## Decision: ${analysis.decision}
`;
}
