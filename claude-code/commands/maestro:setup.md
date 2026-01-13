---
description: Scaffolds the project and sets up the Maestro environment
argument-hint: [no arguments]
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - AskUserQuestion
model: sonnet
---

## 1.0 SYSTEM DIRECTIVE
You are an AI agent. Your primary function is to set up and manage a software project using the Maestro methodology. This document is your operational protocol. Adhere to these instructions precisely and sequentially. Do not make assumptions.

CRITICAL: You must validate the success of every tool call. If any tool call fails, you MUST halt the current operation immediately, announce the failure to the user, and await further instructions.

CRITICAL: When determining model complexity, ALWAYS prefer the "haiku" model for initial exploration and simple tasks, "sonnet" for standard implementation work, and only escalate to "opus" for complex architectural decisions. This ensures efficient token usage while maintaining quality.

**CRITICAL - ASKUSERQUESTION TOOL REQUIREMENT:**
You MUST use the `AskUserQuestion` tool for ALL user interactions including:
- Asking clarifying questions during setup phases
- Presenting options for user selection (A/B/C choices)
- Requesting confirmations and approvals
- Gathering project information (goals, features, tech stack)

DO NOT use plain text output to ask questions. Always use the `AskUserQuestion` tool with properly structured options.

Example usage:
```
AskUserQuestion:
  question: "Which model should be used for setup/status commands?"
  header: "Model"
  options:
    - label: "haiku (recommended)"
      description: "Fast and cost-effective for simple tasks"
    - label: "sonnet"
      description: "Balanced speed and quality"
    - label: "opus"
      description: "Highest quality, slower"
  multiSelect: false
```

---

## 1.1 BEGIN `RESUME` CHECK
**PROTOCOL: Before starting the setup, determine the project's state using the state file.**

1.  **Read State File:** Check for the existence of `maestro/setup_state.json`.
    - If it does not exist, this is a new project setup. Proceed directly to Step 1.2.
    - If it exists, read its content.

2.  **Resume Based on State:**
    - Let the value of `last_successful_step` in the JSON file be `STEP`.
    - Based on the value of `STEP`, jump to the **next logical section**:

    - If `STEP` is "2.1_product_guide", announce "Resuming setup: The Product Guide (`product.md`) is already complete. Next, we will create the Product Guidelines." and proceed to **Section 2.2**.
    - If `STEP` is "2.2_product_guidelines", announce "Resuming setup: The Product Guide and Product Guidelines are complete. Next, we will define the Technology Stack." and proceed to **Section 2.3**.
    - If `STEP` is "2.3_tech_stack", announce "Resuming setup: The Product Guide, Guidelines, and Tech Stack are defined. Next, we will select Code Styleguides." and proceed to **Section 2.4**.
    - If `STEP` is "2.4_code_styleguides", announce "Resuming setup: All guides and the tech stack are configured. Next, we will define the project workflow." and proceed to **Section 2.5**.
    - If `STEP` is "2.5_workflow", announce "Resuming setup: The initial project scaffolding is complete. Next, we will generate the first track." and proceed to **Phase 2 (3.0)**.
    - If `STEP` is "3.3_initial_track_generated":
        - Announce: "The project has already been initialized. You can create a new track with `/maestro:newTrack` or start implementing existing tracks with `/maestro:implement`."
        - Halt the `setup` process.
    - If `STEP` is unrecognized, announce an error and halt.

---

## 1.2 PRE-INITIALIZATION OVERVIEW
1.  **Provide High-Level Overview:**
    -   Present the following overview of the initialization process to the user:
        > "Welcome to Maestro. I will guide you through the following steps to set up your project:
        > 1. **Project Discovery:** Analyze the current directory to determine if this is a new or existing project.
        > 2. **Product Definition:** Collaboratively define the product's vision, design guidelines, and technology stack.
        > 3. **Configuration:** Select appropriate code style guides and customize your development workflow.
        > 4. **Track Generation:** Define the initial track and automatically generate a detailed plan to start development.
        >
        > Let's get started!"

---

## 2.0 PHASE 1: STREAMLINED PROJECT SETUP
**PROTOCOL: Follow this sequence to perform a guided, interactive setup with the user.**


### 2.0 Project Inception
1.  **Detect Project Maturity:**
    -   **Classify Project:** Determine if the project is "Brownfield" (Existing) or "Greenfield" (New) based on the following indicators:
    -   **Brownfield Indicators:**
        -   Check for existence of version control directories: `.git`, `.svn`, or `.hg`.
        -   If a `.git` directory exists, execute `git status --porcelain`. If the output is not empty, classify as "Brownfield" (dirty repository).
        -   Check for dependency manifests: `package.json`, `pom.xml`, `requirements.txt`, `go.mod`.
        -   Check for source code directories: `src/`, `app/`, `lib/` containing code files.
        -   If ANY of the above conditions are met (version control directory, dirty git repo, dependency manifest, or source code directories), classify as **Brownfield**.
    -   **Greenfield Condition:**
        -   Classify as **Greenfield** ONLY if NONE of the "Brownfield Indicators" are found AND the current directory is empty or contains only generic documentation (e.g., a single `README.md` file) without functional code or dependencies.

2.  **Execute Workflow based on Maturity:**
-   **If Brownfield:**
        -   Announce that an existing project has been detected.
        -   If the `git status --porcelain` command (executed as part of Brownfield Indicators) indicated uncommitted changes, inform the user: "WARNING: You have uncommitted changes in your Git repository. Please commit or stash your changes before proceeding, as Maestro will be making modifications."
        -   **Begin Brownfield Project Initialization Protocol:**
            -   **1.0 Pre-analysis Confirmation:**
                1.  **Request Permission:** Inform the user that a brownfield (existing) project has been detected.
                2.  **Ask for Permission:** Request permission for a read-only scan to analyze the project using the `AskUserQuestion` tool:
                    ```
                    AskUserQuestion:
                      question: "Analyze existing project to understand its structure, tech stack, and conventions?"
                      header: "Scan Project"
                      options:
                        - label: "Yes, analyze the project"
                          description: "Perform a read-only scan to understand the codebase"
                        - label: "No, skip analysis"
                          description: "Proceed without analyzing the existing code"
                      multiSelect: false
                    ```
                3.  **Handle Denial:** If permission is denied, halt the process and await further user instructions.
                4.  **Confirmation:** Upon confirmation, proceed to the next step.

            -   **2.0 Code Analysis:**
                1.  **Announce Action:** Inform the user that you will now perform a code analysis.
                2.  **Prioritize README:** Begin by analyzing the `README.md` file, if it exists.
                3.  **Comprehensive Scan:** Extend the analysis to other relevant files to understand the project's purpose, technologies, and conventions.

            -   **2.1 File Size and Relevance Triage:**
                1.  **Respect Ignore Files:** Before scanning any files, you MUST check for the existence of `.geminiignore` and `.gitignore` files. If either or both exist, you MUST use their combined patterns to exclude files and directories from your analysis. The patterns in `.geminiignore` should take precedence over `.gitignore` if there are conflicts. This is the primary mechanism for avoiding token-heavy, irrelevant files like `node_modules`.
                2.  **Efficiently List Relevant Files:** To list the files for analysis, you MUST use a command that respects the ignore files. For example, you can use `git ls-files --exclude-standard -co | xargs -n 1 dirname | sort -u` which lists all relevant directories (tracked by Git, plus other non-ignored files) without listing every single file. If Git is not used, you must construct a `find` command that reads the ignore files and prunes the corresponding paths.
                3.  **Fallback to Manual Ignores:** ONLY if neither `.geminiignore` nor `.gitignore` exist, you should fall back to manually ignoring common directories. Example command: `ls -lR -I 'node_modules' -I '.m2' -I 'build' -I 'dist' -I 'bin' -I 'target' -I '.git' -I '.idea' -I '.vscode'`.
                4.  **Prioritize Key Files:** From the filtered list of files, focus your analysis on high-value, low-size files first, such as `package.json`, `pom.xml`, `requirements.txt`, `go.mod`, and other configuration or manifest files.
                5.  **Handle Large Files:** For any single file over 1MB in your filtered list, DO NOT read the entire file. Instead, read only the first and last 20 lines (using `head` and `tail`) to infer its purpose.

            -   **2.2 Extract and Infer Project Context:**
                1.  **Strict File Access:** DO NOT ask for more files. Base your analysis SOLELY on the provided file snippets and directory structure.
                2.  **Extract Tech Stack:** Analyze the provided content of manifest files to identify:
                    -   Programming Language
                    -   Frameworks (frontend and backend)
                    -   Database Drivers
                3.  **Infer Architecture:** Use the file tree skeleton (top 2 levels) to infer the architecture type (e.g., Monorepo, Microservices, MVC).
                4.  **Infer Project Goal:** Summarize the project's goal in one sentence based strictly on the provided `README.md` header or `package.json` description.

            -   **2.3 Initialize Maestro Directory and Copy Critical Think Templates (EARLY):**
                1.  **Ensure Maestro Directory Exists:** Execute `mkdir -p maestro` to create the Maestro directory if it doesn't exist.
                2.  **Copy Critical Think Templates:** Immediately copy the Critical Think templates to the project so they are available during setup:
                    -   Execute `mkdir -p maestro/critical_think/templates`.
                    -   Copy all Critical Think templates from the user's Maestro installation to the project:
                        -   `~/.claude/maestro-templates/criticalthink_after_action.md` → `maestro/critical_think/templates/criticalthink_after_action.md`
                        -   `~/.claude/maestro-templates/criticalthink_agent_delegation.md` → `maestro/critical_think/templates/criticalthink_agent_delegation.md`
                        -   `~/.claude/maestro-templates/criticalthink_before_action.md` → `maestro/critical_think/templates/criticalthink_before_action.md`
                        -   `~/.claude/maestro-templates/criticalthink_docs.md` → `maestro/critical_think/templates/criticalthink_docs.md`
                        -   `~/.claude/maestro-templates/criticalthink_implementation.md` → `maestro/critical_think/templates/criticalthink_implementation.md`
                        -   `~/.claude/maestro-templates/criticalthink_question.md` → `maestro/critical_think/templates/criticalthink_question.md`
                    -   **Fallback:** If the templates are not found in `~/.claude/maestro-templates/`, copy them from the Maestro installation directory if available, or notify the user that Critical Think templates will need to be added manually.
                3.  **Initialize State File:** Create `maestro/setup_state.json` with the exact content:
                    `{"last_successful_step": ""}`

        -   **Upon completing the brownfield initialization protocol, proceed to the Generate Product Guide section in 2.1.**
    -   **If Greenfield:**
        -   Announce that a new project will be initialized.
        -   Proceed to the next step in this file.

3.  **Initialize Git Repository (for Greenfield):**
    -   If a `.git` directory does not exist, execute `git init` and report to the user that a new Git repository has been initialized.

4.  **Inquire about Project Goal (for Greenfield):**
    -   **Ask the user the following question and wait for their response before proceeding to the next step:** "What do you want to build?"
    -   **CRITICAL: You MUST NOT execute any tool calls until the user has provided a response.**
    -   **Upon receiving the user's response:**
        -   Execute `mkdir -p maestro`.
        -   **Copy Critical Think Templates (EARLY):** Immediately after creating the `maestro` directory, you MUST copy the Critical Think templates to the project so they are available during setup:
            -   Execute `mkdir -p maestro/critical_think/templates`.
            -   Copy all Critical Think templates from the user's Maestro installation to the project:
                -   `~/.claude/maestro-templates/criticalthink_after_action.md` → `maestro/critical_think/templates/criticalthink_after_action.md`
                -   `~/.claude/maestro-templates/criticalthink_agent_delegation.md` → `maestro/critical_think/templates/criticalthink_agent_delegation.md`
                -   `~/.claude/maestro-templates/criticalthink_before_action.md` → `maestro/critical_think/templates/criticalthink_before_action.md`
                -   `~/.claude/maestro-templates/criticalthink_docs.md` → `maestro/critical_think/templates/criticalthink_docs.md`
                -   `~/.claude/maestro-templates/criticalthink_implementation.md` → `maestro/critical_think/templates/criticalthink_implementation.md`
                -   `~/.claude/maestro-templates/criticalthink_question.md` → `maestro/critical_think/templates/criticalthink_question.md`
            -   **Fallback:** If the templates are not found in `~/.claude/maestro-templates/`, copy them from the Maestro installation directory if available, or notify the user that Critical Think templates will need to be added manually.
        -   **Initialize State File:** After copying templates, create `maestro/setup_state.json` with the exact content:
            `{"last_successful_step": ""}`
        -   Write the user's response into `maestro/product.md` under a header named `# Initial Concept`.

5.  **Continue:** Immediately proceed to the next section.

### 2.1 Generate Product Guide (Interactive)
1.  **Introduce the Section:** Announce that you will now help the user create the `product.md`.
2.  **Ask Questions Sequentially:** Ask one question at a time using the `AskUserQuestion` tool. Wait for and process the user's response before asking the next question. Continue this interactive process until you have gathered enough information.
        -   **CONSTRAINT:** Limit your inquiry to a maximum of 5 questions.
        -   **SUGGESTIONS:** For each question, generate 3 high-quality suggested answers based on common patterns or context you already have.
        -   **Example Topics:** Target users, goals, features, etc
        *   **General Guidelines:**
            *   **1. Classify Question Type:** Before formulating any question, you MUST first classify its purpose as either "Additive" or "Exclusive Choice".
                *   Use **Additive** for brainstorming and defining scope (e.g., users, goals, features, project guidelines). These questions allow for multiple answers.
                *   Use **Exclusive Choice** for foundational, singular commitments (e.g., selecting a primary technology, a specific workflow rule). These questions require a single answer.

            *   **2. Formulate the Question:** Based on the classification, you MUST use the `AskUserQuestion` tool with proper structure:
                ```
                AskUserQuestion:
                  question: "Your question here?"
                  header: "Short Header"
                  options:
                    - label: "Option A"
                      description: "Brief description of option A"
                    - label: "Option B"
                      description: "Brief description of option B"
                    - label: "Option C"
                      description: "Brief description of option C"
                    - label: "Type your own answer"
                      description: "Provide a custom response"
                    - label: "Autogenerate and review product.md"
                      description: "Auto-generate the remaining content and proceed"
                  multiSelect: false  # or true for additive questions
                ```
            *   **3. Interaction Flow:**
                    *   **CRITICAL:** You MUST ask questions sequentially (one by one). Do not ask multiple questions in a single turn. Wait for the user's response after each question.
                *   The last two options for every multiple-choice question MUST be "Type your own answer" and "Autogenerate and review product.md".
                *   Confirm your understanding by summarizing before moving on.
    -   **FOR EXISTING PROJECTS (BROWNFIELD):** Ask project context-aware questions based on the code analysis.
    -   **AUTO-GENERATE LOGIC:** If the user selects "Autogenerate and review product.md", immediately stop asking questions for this section. Use your best judgment to infer the remaining details based on previous answers and project context, generate the full `product.md` content, write it to the file, and proceed to the next section.
3.  **Apply Prompt Enhancer:** Before generating the document, you MUST apply the user's "prompt enhancer" hook if available. This hook enhances question generation and response synthesis based on user preferences and context.
4.  **Draft the Document:** Once the dialogue is complete (or auto-generate is selected), generate the content for `product.md`. If auto-generate was chosen, use your best judgment to infer the remaining details based on previous answers and project context. You are encouraged to expand on the gathered details to create a comprehensive document.
    -   **CRITICAL:** The source of truth for generation is **only the user's selected answer(s)**. You MUST completely ignore the questions you asked and any of the unselected options you presented.
        -   **Action:** Take the user's chosen answer and synthesize it into a well-formed section for the document. You are encouraged to expand on the user's choice to create a comprehensive and polished output. DO NOT include the conversational options in the final file.
5.  **User Confirmation Loop:** Present the drafted content to the user for review and begin the confirmation loop using `AskUserQuestion`:
    ```
    AskUserQuestion:
      question: "I've drafted the product guide based on your responses. Please review and decide:"
      header: "Review Draft"
      options:
        - label: "Approve"
          description: "The document is correct and we can proceed"
        - label: "Suggest Changes"
          description: "Tell me what to modify (you can also edit directly after this step)"
      multiSelect: false
    ```
    -   **Loop:** Based on user response, either apply changes and re-present the document, or break the loop on approval.
6.  **Write File:** Once approved, append the generated content to the existing `maestro/product.md` file, preserving the `# Initial Concept` section.
7.  **Commit State:** Upon successful creation of the file, you MUST immediately write to `maestro/setup_state.json` with the exact content:
    `{"last_successful_step": "2.1_product_guide"}`
8.  **Continue:** After writing the state file, immediately proceed to the next section.

### 2.2 Generate Product Guidelines (Interactive)
1.  **Introduce the Section:** Announce that you will now help the user create the `product-guidelines.md`.
2.  **Ask Questions Sequentially:** Ask one question at a time using the `AskUserQuestion` tool. Wait for and process the user's response before asking the next question. Continue this interactive process until you have gathered enough information.
    -   **CONSTRAINT:** Limit your inquiry to a maximum of 5 questions.
    -   **SUGGESTIONS:** For each question, generate 3 high-quality suggested answers based on common patterns or context you already have. Provide a brief rationale for each and highlight the one you recommend most strongly.
    -   **Example Topics:** Prose style, brand messaging, visual identity, etc
    *   **General Guidelines:** Use the same `AskUserQuestion` format as in section 2.1, adjusting the question header and options appropriately. Include "Autogenerate and review product-guidelines.md" as the final option.
    -   **AUTO-GENERATE LOGIC:** If the user selects "Autogenerate and review product-guidelines.md", immediately stop asking questions for this section and proceed to draft the document.
3.  **Apply Prompt Enhancer:** Before generating the document, you MUST apply the user's "prompt enhancer" hook if available.
4.  **Draft the Document:** Once the dialogue is complete (or auto-generate is selected), generate the content for `product-guidelines.md`. Use the same source-of-truth principles as in section 2.1.
5.  **User Confirmation Loop:** Present the drafted content to the user for review using `AskUserQuestion`:
    ```
    AskUserQuestion:
      question: "I've drafted the product guidelines based on your responses. Please review and decide:"
      header: "Review Draft"
      options:
        - label: "Approve"
          description: "The document is correct and we can proceed"
        - label: "Suggest Changes"
          description: "Tell me what to modify (you can also edit directly after this step)"
      multiSelect: false
    ```
    -   **Loop:** Based on user response, either apply changes and re-present the document, or break the loop on approval.
6.  **Write File:** Once approved, write the generated content to the `maestro/product-guidelines.md` file.
7.  **Commit State:** Upon successful creation of the file, you MUST immediately write to `maestro/setup_state.json` with the exact content:
    `{"last_successful_step": "2.2_product_guidelines"}`
8.  **Continue:** After writing the state file, immediately proceed to the next section.

### 2.3 Generate Tech Stack (Interactive)
1.  **Introduce the Section:** Announce that you will now help define the technology stacks.
2.  **Ask Questions Sequentially:** Ask one question at a time using the `AskUserQuestion` tool. Wait for and process the user's response before asking the next question. Continue this interactive process until you have gathered enough information.
    -   **CONSTRAINT:** Limit your inquiry to a maximum of 5 questions.
    -   **SUGGESTIONS:** For each question, generate 3 high-quality suggested answers based on common patterns or context you already have.
    -   **Example Topics:** programming languages, frameworks, databases, etc
    *   **General Guidelines:** Use the same `AskUserQuestion` format as in section 2.1, adjusting the question header and options appropriately.
    -   **FOR EXISTING PROJECTS (BROWNFIELD):**
            -   **CRITICAL WARNING:** Your goal is to document the project's *existing* tech stack, not to propose changes.
            -   **State the Inferred Stack:** Based on the code analysis, you MUST state the technology stack that you have inferred. Do not present any other options.
            -   **Request Confirmation:** After stating the detected stack, you MUST ask the user for confirmation using `AskUserQuestion`:
                ```
                AskUserQuestion:
                  question: "Based on my analysis, your project uses: [inferred stack]. Is this correct?"
                  header: "Confirm Stack"
                  options:
                    - label: "Yes, this is correct"
                      description: "The inferred tech stack is accurate"
                    - label: "No, I need to provide corrections"
                      description: "I will provide the correct tech stack"
                  multiSelect: false
                ```
            -   **Handle Disagreement:** If the user indicates the stack is incorrect, allow them to provide the correct technology stack.
    -   **AUTO-GENERATE LOGIC:** If the user selects "Autogenerate and review tech-stack.md", immediately stop asking questions and proceed to draft the document.
3.  **Apply Prompt Enhancer:** Before generating the document, you MUST apply the user's "prompt enhancer" hook if available.
4.  **Draft the Document:** Once the dialogue is complete (or auto-generate is selected), generate the content for `tech-stack.md`. Use the same source-of-truth principles as in section 2.1.
5.  **User Confirmation Loop:** Present the drafted content to the user for review using `AskUserQuestion`:
    ```
    AskUserQuestion:
      question: "I've drafted the tech stack document based on your responses. Please review and decide:"
      header: "Review Draft"
      options:
        - label: "Approve"
          description: "The document is correct and we can proceed"
        - label: "Suggest Changes"
          description: "Tell me what to modify (you can also edit directly after this step)"
      multiSelect: false
    ```
    -   **Loop:** Based on user response, either apply changes and re-present the document, or break the loop on approval.
6.  **Write File:** Once approved, write the generated content to the `maestro/tech-stack.md` file.
7.  **Commit State:** Upon successful creation of the file, you MUST immediately write to `maestro/setup_state.json` with the exact content:
    `{"last_successful_step": "2.3_tech_stack"}`
8.  **Continue:** After writing the state file, immediately proceed to the next section.

### 2.4 Select Guides (Interactive)
1.  **Initiate Dialogue:** Announce that the initial scaffolding is complete and you now need the user's input to select the project's guides from the locally available templates.
2.  **Select Code Style Guides:**
    -   List the available style guides by running `ls ~/.claude/maestro-templates/code_styleguides/`.
    -   For new projects (greenfield):
        -   **Recommendation:** Based on the Tech Stack defined in the previous step, recommend the most appropriate style guide(s) and explain why.
        -   Ask the user using `AskUserQuestion`:
            ```
            AskUserQuestion:
              question: "Based on your tech stack, I recommend these style guides: [list]. How would you like to proceed?"
              header: "Style Guides"
              options:
                - label: "Include the recommended style guides"
                  description: "Use the recommended guides for this project"
                - label: "Edit the selected set"
                  description: "Choose different style guides from the available options"
              multiSelect: false
            ```
        -   If the user chooses to edit:
            -   Present the list of all available guides to the user as a **numbered list**.
            -   Ask the user which guide(s) they would like to copy.
    -   For existing projects (brownfield):
        -   **Announce Selection:** Inform the user: "Based on the inferred tech stack, I will copy the following code style guides: <list of inferred guides>."
        -   **Ask for Customization:** Ask the user using `AskUserQuestion`:
            ```
            AskUserQuestion:
              question: "Based on your tech stack, I recommend these style guides: [list]. Proceed with these or add more?"
              header: "Style Guides"
              options:
                - label: "Yes, proceed with suggested guides"
                  description: "Use the recommended guides for this project"
                - label: "No, add more style guides"
                  description: "Choose additional style guides from the available options"
              multiSelect: false
            ```
    -   **Action:** Construct and execute a command to create the directory and copy all selected files. For example: `mkdir -p maestro/code_styleguides && cp ~/.claude/maestro-templates/code_styleguides/python.md ~/.claude/maestro-templates/code_styleguides/javascript.md maestro/code_styleguides/`
    -   **Commit State:** Upon successful completion of the copy command, you MUST immediately write to `maestro/setup_state.json` with the exact content:
        `{"last_successful_step": "2.4_code_styleguides"}`

### 2.5 Select Workflow and Configure Autonomous Mode (Interactive)
1.  **Copy Initial Workflow:**
    -   Copy `~/.claude/maestro-templates/workflow.md` to `maestro/workflow.md`.

2.  **Configure Workflow Mode:**
    -   **Ask the user** using `AskUserQuestion`:
        ```
        AskUserQuestion:
          question: "Do you want to use the default manual workflow or autonomous mode?"
          header: "Workflow Mode"
          options:
            - label: "Default Manual Workflow"
              description: "Pause for user verification after each phase completion"
            - label: "Autonomous - Full"
              description: "Pause only at final phase (full autonomy)"
            - label: "Autonomous - Checkpoints (33%, 66%, 99%)"
              description: "Pause at every 3rd phase completion"
            - label: "Autonomous - Checkpoints (25%, 50%, 75%, 100%)"
              description: "Pause at every quarter completion point"
            - label: "Autonomous - Checkpoints (50%, 100%)"
              description: "Pause at every half completion point"
          multiSelect: false
        ```

3.  **Configure "Tzar of Excellence" Review Agent:**
    -   **Inform the user:** "The 'Tzar of Excellence' is a rigorous zero-tolerance code review that ensures production-ready quality before proceeding to the next phase."
    -   **Ask the user** using `AskUserQuestion`:
        ```
        AskUserQuestion:
          question: "Which agent should conduct the 'Tzar of Excellence' review for each phase?"
          header: "Review Agent"
          options:
            - label: "codex-reviewer (recommended)"
              description: "GPT-5 reasoning, high-rigor production review"
            - label: "gemini-analyzer"
              description: "1M+ context, comprehensive analysis"
            - label: "opus-specialist"
              description: "Advanced reasoning with thinking mode"
            - label: "qwen-coder"
              description: "Production implementation focus"
            - label: "Type custom agent name"
              description: "Specify a different agent"
          multiSelect: false
        ```

4.  **Create Workflow Configuration File:**
    -   Create `maestro/workflow-config.json` with the selected configuration:
        ```json
        {
          "workflow_mode": "manual",
          "checkpoint_interval": null,
          "review_agent": "codex-reviewer",
          "review_criteria": {
            "zero_tolerance": true,
            "check_security": true,
            "check_edge_cases": true,
            "check_error_handling": true,
            "check_performance": true,
            "min_code_coverage": 95
          }
        }
        ```
    -   **If autonomous mode selected:** Set `workflow_mode` to `"autonomous"` and `checkpoint_interval` to the selected value.
    -   **Set `review_agent`** to the selected agent name.

5.  **Commit State:** After the `workflow.md` file and `workflow-config.json` are successfully written or updated, you MUST immediately write to `maestro/setup_state.json` with the exact content:
    `{"last_successful_step": "2.5_workflow"}`

### 2.6 Configure claude-hud Integration (Interactive)
1.  **Introduce claude-hud:**
    -   **Explain:** "claude-hud provides native token counting and cost estimation in your Claude Code statusline. It shows real-time token usage, cost estimates, and session statistics during Maestro work."
    -   **Benefits:**
        - No separate API calls needed for tracking
        - Native integration with Claude Code session
        - Real-time feedback on token usage
        - Cost estimates for budget management
        - Eliminates need for custom cost tracking

2.  **Check Installation Status:**
    -   Run: `which claude-hud` or check if claude-hud command is available
    -   If installed: Skip to step 4 (configure statusline)
    -   If not installed: Proceed to step 3

3.  **Offer Installation:**
    -   **Ask** using `AskUserQuestion`:
        ```
        AskUserQuestion:
          question: "claude-hud is not installed. Would you like to install it now?"
          header: "Install claude-hud"
          options:
            - label: "Yes, install claude-hud"
              description: "Install claude-hud for native token tracking (recommended)"
            - label: "Skip for now"
              description: "Can install later with /claude-hud:setup"
          multiSelect: false
        ```
    -   **If user selects to install:**
        -   Run: `/claude-hud:setup` (if available as command)
        -   Or provide manual installation instructions.
    -   Verify installation and report status

4.  **Configure Statusline:**
    -   **Ask** using `AskUserQuestion`:
        ```
        AskUserQuestion:
          question: "Configure statusline to show Maestro session information?"
          header: "Statusline Config"
          options:
            - label: "Yes, configure for Maestro"
              description: "Show Maestro-specific info in statusline (recommended)"
            - label: "Use default settings"
              description: "Use standard claude-hud configuration"
          multiSelect: false
        ```
    -   **If user selects to configure:**
        -   Create or update claude-hud configuration to show Maestro-specific information.
        -   Explain: "claude-hud will now show Maestro-specific information in your statusline"

5.  **Document Configuration:**
    -   Add claude-hud configuration to project notes:
        -   Create `maestro/.claude-hud.md` with:
            ```markdown
            # claude-hud Configuration for Maestro

            This project uses claude-hud for native token tracking.

            **Installation:** Installed / Not Installed
            **Statusline Configured:** Yes / No
            **Configuration Date:** <date>

            ## Statusline Features

            - Shows current Maestro command
            - Displays track/task context
            - Real-time token usage
            - Cost estimates

            ## Notes

            claude-hud provides native tracking without separate API calls.
            See https://github.com/Cline-org/claude-hud for more information.
            ```

6.  **Commit State:** After claude-hud configuration is complete, write to `maestro/setup_state.json` with the exact content:
    `{"last_successful_step": "2.6_claude_hud"}`

### 2.7 Finalization
1.  **Summarize Actions:** Present a summary of all actions taken during Phase 1, including:
    -   The guide files that were copied.
    -   The workflow file that was copied.
    -   The workflow mode that was configured.
    -   The "Tzar of Excellence" review agent that was selected.
    -   The claude-hud integration status.
2.  **Transition to initial plan and track generation:** Announce that the initial setup is complete and you will now proceed to define the first track for the project.

---

## 3.0 INITIAL PLAN AND TRACK GENERATION
**PROTOCOL: Interactively define project requirements, propose a single track, and then automatically create the corresponding track and its phased plan.**

### 3.1 Generate Product Requirements (Interactive)(For greenfield projects only)
1.  **Transition to Requirements:** Announce that the initial project setup is complete. State that you will now begin defining the high-level product requirements by asking about topics like user stories and functional/non-functional requirements.
2.  **Analyze Context:** Read and analyze the content of `maestro/product.md` to understand the project's core concept.
3.  **Ask Questions Sequentially:** Ask one question at a time using the `AskUserQuestion` tool. Wait for and process the user's response before asking the next question. Continue this interactive process until you have gathered enough information.
    -   **CONSTRAINT** Limit your inquiries to a maximum of 5 questions.
    -   **SUGGESTIONS:** For each question, generate 3 high-quality suggested answers based on common patterns or context you already have.
    *   **General Guidelines:** Use the same `AskUserQuestion` format as in section 2.1, adjusting the question header and options appropriately. Include "Auto-generate the rest of requirements and move to the next step" as the final option.
    -   **AUTO-GENERATE LOGIC:** If the user selects the auto-generate option, immediately stop asking questions and proceed to the next section.
    -   **CRITICAL:** When processing user responses, the source of truth for generation is **only the user's selected answer(s)**. You MUST completely ignore the questions you asked and any of the unselected options you presented.
4.  **Continue:** After gathering enough information, immediately proceed to the next section.

### 3.2 Propose a Single Initial Track (Automated + Approval)
1.  **State Your Goal:** Announce that you will now propose an initial track to get the project started.
2.  **Generate Track Title:** Analyze the project context (`product.md`, `tech-stack.md`) and (for greenfield projects) the requirements gathered in the previous step. Generate a single track title that summarizes the entire initial track. For existing projects (brownfield): Recommend a plan focused on maintenance and targeted enhancements that reflect the project's current state.
3.  **User Confirmation:** Present the generated track title to the user for review and approval using `AskUserQuestion`:
    ```
    AskUserQuestion:
      question: "I propose this initial track: [track description]. Does this look correct?"
      header: "Confirm Track"
      options:
        - label: "Yes, proceed with this track"
          description: "The track description is accurate"
        - label: "No, I want to modify it"
          description: "Provide a different track description"
      multiSelect: false
    ```
    -   If the user selects to modify, ask them for clarification on what track to start with.

### 3.3 Convert the Initial Track into Artifacts (Automated)
1.  **State Your Goal:** Once the track is approved, announce that you will now create the artifacts for this initial track.
2.  **Initialize Tracks File:** Create the `maestro/tracks.md` file with the initial header and the first track:
    ```markdown
    # Project Tracks

    This file tracks all major tracks for the project. Each track has its own detailed plan in its respective folder.

    ---

    ## [ ] Track: <Track Description>
    *Link: [./maestro/tracks/<track_id>/](./maestro/tracks/<track_id>/)*
    ```
3.  **Generate Track Artifacts:**
    a. **Define Track:** The approved title is the track description.
    b. **Generate Track-Specific Spec & Plan:**
        i. Automatically generate a detailed `spec.md` for this track.
        ii. Automatically generate a `plan.md` for this track.
            - **CRITICAL:** The structure of the tasks must adhere to the principles outlined in the workflow file at `maestro/workflow.md`. For example, if the workflow specifies Test-Driven Development, each feature task must be broken down into a "Write Tests" sub-task followed by an "Implement Feature" sub-task.
            - **CRITICAL: Inject Phase Completion Tasks.** You MUST read the `maestro/workflow.md` file to determine if a "Phase Completion Verification and Checkpointing Protocol" is defined. If this protocol exists, then for each **Phase** that you generate in `plan.md`, you MUST append a final meta-task to that phase. The format for this meta-task is: `- [ ] Task: Maestro - Phase Verification and Checkpoint '<Phase Name>' (Protocol in workflow.md)`. You MUST replace `<Phase Name>` with the actual name of the phase.
    c. **Create Track Artifacts:**
        i. **Generate and Store Track ID:** Create a unique Track ID from the track description using format `shortname_YYYYMMDD` and store it. You MUST use this exact same ID for all subsequent steps for this track.
        ii. **Create Single Directory:** Using the stored Track ID, create a single new directory: `maestro/tracks/<track_id>/`.
        iii. **Create `metadata.json`:** In the new directory, create a `metadata.json` file with the correct structure and content, using the stored Track ID. An example is:
            - ```json
            {
            "track_id": "<track_id>",
            "type": "feature", // or "bug"
            "status": "new", // or in_progress, completed, cancelled
            "created_at": "YYYY-MM-DDTHH:MM:SSZ",
            "updated_at": "YYYY-MM-DDTHH:MM:SSZ",
            "description": "<Initial user description>",
            }
            ```
        Populate fields with actual values. Use the current timestamp.
        iv. **Write Spec and Plan Files:** In the exact same directory, write the generated `spec.md` and `plan.md` files.

    d. **Commit State:** After all track artifacts have been successfully written, you MUST immediately write to `maestro/setup_state.json` with the exact content:
       `{"last_successful_step": "3.3_initial_track_generated"}`

    e. **Announce Progress:** Announce that the track for "<Track Description>" has been created.

### 3.4 Final Announcement
1.  **Announce Completion:** After the track has been created, announce that the project setup and initial track generation are complete.
2.  **Save Maestro Files:** Add and commit all files with the commit message `maestro(setup): Add maestro setup files`.
3.  **Store Setup Memory:** Store the maestro environment setup in Nexus memory:
    - Store project context (product.md, tech-stack.md summary)
    - Store workflow preferences (workflow mode, review agent)
    - Store maestro initialization timestamp
4.  **Next Steps:** Inform the user that they can now begin work by running `/maestro:implement`.
