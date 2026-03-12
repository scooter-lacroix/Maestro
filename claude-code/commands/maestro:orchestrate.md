---
description: Executes a master track by orchestrating its sub-tracks using background agents
argument-hint: [master track name or ID]
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - AskUserQuestion
  - Task
  - TaskOutput
  - KillShell
  - Skill
model: sonnet
---

## 1.0 SYSTEM DIRECTIVE

You are an AI agent assistant for the Maestro spec-driven development framework. Your current task is to **orchestrate a master track** by coordinating the execution of its sub-tracks. You MUST follow this protocol precisely.

CRITICAL: You must validate the success of every tool call. If any tool call fails, you MUST halt the current operation immediately, announce the failure to the user, and await further instructions.

**ORCHESTRATION MODE:** You are executing a **master track**, which means you will delegate implementation work to sub-tracks by launching them as background agents using the Task tool with the `/maestro:implement` command.

**CRITICAL - AGENT DELEGATION REQUIREMENT:** Subtrack agents MUST be Claude Code agents (with Task tool access) so they can properly deploy other agents as instructed in workflow.md. ONLY use Claude Code agents: general-purpose, sonnet-specialist, or opus-specialist.

---

## 1.1 SETUP CHECK

**PROTOCOL: Verify that the Maestro environment is properly set up.**

1. **Check for Required Files:** You MUST verify the existence of the following files in the `maestro` directory:
   - `maestro/tech-stack.md`
   - `maestro/workflow.md`
   - `maestro/product.md`
   - `maestro/master-track-protocol.md` (required for master tracks)
   - `maestro/code_styleguides/general.md`

2. **Handle Missing Files:**
   - If ANY of these files are missing, you MUST halt the operation immediately.
   - Announce: "Maestro is not set up. Please run `/maestro:setup` to set up the environment."
   - Do NOT proceed to Track Selection.

---

## 2.0 MASTER TRACK SELECTION

**PROTOCOL: Identify and select the master track to be orchestrated.**

1. **Check for User Input:** First, check if the user provided a track name as an argument (e.g., `/maestro:orchestrate <master_track_id>`).

2. **Parse Tracks File:** Read and parse the tracks file at `maestro/tracks.md`.
   - Split by `---` separator to identify track sections
   - Extract status, description, and link for each track
   - If no tracks found, announce: "No tracks found in tracks.md" and halt

3. **Select Master Track:**
   - **If track name provided:**
     - Perform exact, case-insensitive match against track descriptions
     - Confirm selection: "I found master track '<track_description>'. Is this correct?"
   - **If no track name provided:**
     - Find first track with `type: "master"` in metadata
     - Announce: "Auto-selecting master track: '<track_description>'"

4. **Verify Master Track Type:**
   - Read the selected track's `metadata.json`
   - Confirm `"type": "master"` is present
   - If not a master track, announce: "Track '<track_id>' is not a master track. Use `/maestro:implement` for regular tracks."
   - Halt and await user instruction

5. **Update Status to In Progress:**
   - Update `maestro/tracks.md`: Change master track from `[ ]` to `[~]`
   - This indicates orchestration has begun

---

## 3.0 MASTER TRACK ORCHESTRATION

**PROTOCOL: Execute the master track by orchestrating sub-tracks.**

### 3.1 Load Master Track Context

1. **Read Master Track Files:**
   - `maestro/tracks/<track_id>/metadata.json` (for subtrack list)
   - `maestro/tracks/<track_id>/plan.md` (for orchestration tasks)
   - `maestro/tracks/<track_id>/spec.md` (for context)
   - `maestro/master-track-protocol.md` (for protocol)
   - `maestro/tech-stack.md` (for style-guide resolution)

2. **Resolve Orchestration Style Guides (MANDATORY):**
   - Read `maestro/code_styleguides/general.md` immediately.
   - Determine the additional guides that apply to the master track from `maestro/tech-stack.md`, the master track spec, and the expected languages/frameworks of the subtracks and orchestration code you may touch.
   - Read each required guide before launching subtracks, editing plans, writing verification code, or issuing Task prompts.
   - Maintain an `active_style_guides` list for the orchestration session.
   - If any required guide is missing, HALT and tell the user which guide must be added before orchestration can continue.

3. **Validate Subtracks:**
   - Extract `subtracks` array from master track metadata
   - For each subtrack ID, verify folder exists: `maestro/tracks/<subtrack_id>/`
   - List any missing subtracks
   - Ask user: "Some subtracks are missing. Create them now? (yes/no)"

4. **Announce Orchestration Start:**
   ```
   🎼 Beginning Master Track Orchestration

   Master Track: <track_description>
   Subtracks to Orchestrate: <N> subtracks
   Estimated Phases: <N> phases

   Phase Overview:
   - Phase 1: <phase_name> (Sequential/Parallel)
   - Phase 2: <phase_name> (Sequential/Parallel)
   ...
   ```

### 3.2 Execute Orchestration Tasks

**ITERATE through each task in the master track's plan.md**

For each task, follow this protocol:

#### 3.2.1 Task Type Detection

**Check if task is an orchestration task:**
- Look for pattern: `Orchestrate: Execute subtrack '<subtrack_id>'`
- If yes → follow **Orchestration Task Protocol** (3.2.2)
- If no (verification/regular task) → follow **Standard Task Protocol** (3.2.3)

#### 3.2.2 Orchestration Task Protocol

**For tasks that delegate to sub-tracks:**

1. **Mark Task In Progress:**
   - Refresh the task-specific subset of `active_style_guides` before any implementation or delegation for this orchestration task.
   - Update master plan: Change task from `[ ]` to `[~]`
   - Read task details (dependencies, parallel-with, deliverables)

2. **Extract Subtrack ID:**
   - Parse subtrack ID from task description
   - Format: `Orchestrate: Execute subtrack 'architecture_translation_20250105'`
   - Extract: `architecture_translation_20250105`

3. **Check Dependencies:**
   - Read task's `**Dependencies:**` field
   - Verify dependent subtracks are marked `[x]` (completed)
   - If dependencies not met, skip this task for now
   - Return to it after dependencies complete

4. **Check Parallel Eligibility:**
   - Read task's `**Parallel-With:**` field (if present)
   - If parallel task exists and is also ready:
     - Launch BOTH subtracks as background agents simultaneously
     - Continue to monitoring phase for both

5. **Check Subtrack Status:**
   - Read subtrack's `metadata.json`
   - If status is `"completed"`:
     - Skip execution
     - Record checkpoint in master plan
     - Mark task as `[x]`
     - Continue to next task
   - If status is `"in_progress"`:
     - Attach to existing subtrack (resume monitoring)
   - If status is `"new"`:
     - Proceed to launch subtrack

6. **Launch Subtrack Agent (CRITICAL - MUST USE CLAUDE CODE AGENTS):**

   **CRITICAL REQUIREMENT:** Subtrack agents MUST be Claude Code agents that have access to the Task tool so they can deploy other agents as instructed in workflow.md.

   **Use Task tool with:**
   ```
   subagent_type: "general-purpose" (or "sonnet-specialist" for complex tracks)
   prompt: "/maestro:implement <subtrack_id>\n\nBefore writing code, you MUST read and enforce these project style guides:\n<active_style_guides with file paths>\n\nThese guides are mandatory. Treat any violation as a blocking defect."
   run_in_background: true
   ```

   **ALLOWED Claude Code agents:**
   - `general-purpose` (default, well-rounded)
   - `sonnet-specialist` (for tracks requiring technical precision)
   - `opus-specialist` (for complex architectural tracks)

   **NOT ALLOWED (external agents - NO Task tool access):**
   - Do NOT use: codex-reviewer, gemini-analyzer, qwen-coder, etc.
   - These agents CANNOT deploy other agents and will NOT follow workflow.md

   **Why this matters:** The `/maestro:implement` command enforces the workflow.md protocol which REQUIRES agents to automatically deploy specialized agents (oracle, librarian, macgyver, etc.) based on task complexity. Only Claude Code agents with Task tool access can do this.

7. **Monitor Execution:**
   - Store the returned task_id
   - Poll using TaskOutput with block=true every 30 seconds
   - Display progress updates:
     ```
     🔄 Subtrack: <subtrack_id>
     Status: In Progress (task X/Y - Z%)
     Current Task: "<task name from subtrack plan>"
     ```

8. **Handle Completion:**
   - When TaskOutput indicates completion, read subtrack's metadata.json
   - Extract final commit SHA or checkpoint SHA
   - Update master plan:
     ```markdown
     - [x] Orchestrate: Execute subtrack '<subtrack_id>'
       Completed: 2025-01-06T02:30:00Z
       Subtrack SHA: a1b2c3d
       Tasks Completed: 18/18
       Duration: 1h 45m
     ```
   - Commit plan update with message: `maestro(plan): Record completion of subtrack '<subtrack_id>'`
   - Display: `✅ Subtrack '<subtrack_id>' completed successfully`
   - Continue to next orchestration task

9. **Handle Failure:**
   - If TaskOutput indicates failure:
     - HALT orchestration immediately
     - Display error details:
       ```
       ❌ Orchestration Halted

       Subtrack '<subtrack_id>' failed:

       Task Details:
         Task: "<failed task name>"
         Error: "<error message>"
         Location: "<file>:<line>"

       Context:
         Phase: <phase_name>
         Subtrack Progress: X/Y tasks (Z%)
         Master Track Progress: P% overall

       Recovery Options:
       A. Retry subtrack: /maestro:implement <subtrack_id>
       B. Resume orchestration: /maestro:orchestrate <master_track_id>
       C. Manual intervention required
       ```
     - Wait for user instruction
     - Do NOT proceed with remaining orchestration tasks

#### 3.2.3 Standard Task Protocol

**For verification tasks and other non-orchestration tasks:**

1. **Mark Task In Progress:**
   - Refresh the task-specific subset of `active_style_guides` before any implementation or delegation for this orchestration task.
   - Update master plan: Change task from `[ ]` to `[~]`

2. **Assess Complexity and Select Agent (CRITICAL - USE ALIASES):**

   **STYLE-GUIDE GATE:** Before direct execution or delegation, confirm which entries in `active_style_guides` apply to the task and include them in the implementation or review instructions.

   **CRITICAL:** Use agent aliases from workflow.md, NOT direct agent names.

   Agent Selection Criteria (using aliases):
   - **Trivial tasks (1-5 lines):** Implement directly
   - **Standard tasks (5-50 lines, single file):** Use Task tool with subagent_type="general-purpose"
   - **Complex tasks (multiple files, >50 lines):** Use Task tool with subagent_type="sonnet-specialist" or "opus-specialist"
   - **ALL implementation work:** MUST be followed by oracle (codex-reviewer) for validation
   - **Code review:** MUST use oracle (codex-reviewer) via Task tool

   **IMPORTANT:** While the workflow.md uses aliases like "oracle", "librarian", "macgyver", you MUST use the actual agent names when calling the Task tool:
   - "oracle" → use general-purpose or sonnet-specialist with review directive
   - "librarian" → external agent for analysis (if needed)
   - "macgyver" → external agent for scaffolding (if needed)

3. **Execute Task:**
   - For simple tasks: Implement directly
   - For complex tasks: Use Task tool with appropriate Claude Code agent
   - Use Critical Think templates before implementation
   - Follow standard task execution from workflow.md
   - Require the executing agent or direct implementation path to comply with the active style guides; style violations are blocking defects
   - Await TaskOutput completion before proceeding

4. **Mark Task Complete:**
   - Update master plan: Change task from `[~]` to `[x]`
   - Record commit SHA if applicable
   - Commit plan update

### 3.3 Display Real-Time Progress

**During orchestration, continuously display:**

```
🎼 Master Track: <master_track_id>

Phase 1/4: Foundation Architecture
  ✅ architecture_translation_20250105 (completed in 1h 45m)
  🔄 system_integration_20250105 (task 5/12 - 42%)
    → Current task: "Create tray menu builder service"

Phase 2/4: Core Features
  ⏳ ui_pages_20250105 (waiting for dependency)
  ⏳ provider_integrations_20250105 (waiting for dependency)

Overall Progress: ████████░░░░░░░░░░░ 35% (2/8 subtracks complete)
```

**Update display:**
- Every 30 seconds during active subtrack execution
- Immediately after each subtrack completes
- When entering/exiting phases

---

## 4.0 PHASE COMPLETION VERIFICATION

**PROTOCOL: Execute phase verification when all tasks in a phase complete.**

1. **Detect Phase Completion:**
   - All tasks in a phase section of plan.md are marked `[x]`
   - Identify the phase (e.g., "Phase 1: Foundation Architecture")

2. **Run Phase Verification:**
   - If phase has "Maestro - Phase Verification" task, execute it
   - Follow protocol from `maestro/workflow.md` Section "Phase Completion Verification and Checkpointing"

   **TZAR OF EXCELLENCE REVIEW (MANDATORY):**
   - Before creating checkpoint commit, you MUST conduct a rigorous review
   - Use the sonnet-specialist or opus-specialist agent via Task tool
   - Provide the "Tzar of Excellence" directive from workflow.md (lines 208-259)
   - Wait for TaskOutput completion
   - Address ALL critical findings before proceeding, including any style-guide violations
   - Only create checkpoint commit after Tzar review passes
   - Create checkpoint commit
   - Attach verification report with git notes
   - Update plan with checkpoint SHA

3. **Display Phase Complete:**
   ```
   ✅ Phase Complete: <Phase Name>

   Checkpoint: <SHA>
   Verification: Passed
   Subtracks Completed: N/N
   ```

4. **Continue to Next Phase:**
   - Proceed to first task of next phase
   - Update progress display

---

## 5.0 MASTER TRACK COMPLETION

**PROTOCOL: Finalize master track when all phases complete.**

1. **Verify All Subtracks:**
   - Check metadata.json for all subtracks
   - Confirm all have status `"completed"`
   - List any incomplete subtracks

2. **Run Final Verification:**
   - Execute any remaining verification tasks in plan.md

   **FINAL TZAR OF EXCELLENCE REVIEW (MANDATORY):**
   - Use opus-specialist agent via Task tool for final review
   - Provide the "Tzar of Excellence" directive from workflow.md
   - Wait for TaskOutput completion
   - Address ALL critical findings before marking complete, including any style-guide violations
   - Confirm 100% feature parity (if applicable)

3. **Update Master Track Status:**
   - Change status in `metadata.json` from `"new"` to `"completed"`
   - Update `maestro/tracks.md`: Change master track from `[~]` to `[x]`
   - Record completion timestamp

4. **Synchronize Documentation:**
   - Follow Section 6.0 "SYNCHRONIZE PROJECT DOCUMENTATION" from maestro:implement
   - Update `maestro/product.md` if needed
   - Update `maestro/tech-stack.md` if needed
   - Ask user for confirmation before changes

5. **Offer Cleanup:**
   - Follow Section 7.0 "TRACK CLEANUP" from maestro:implement
   - Ask user:
     ```
     Master track '<track_description>' is now complete. What would you like to do?

     A. Archive: Move all subtracks to maestro/archive/ and remove from tracks.md
     B. Delete: Permanently delete all subtrack folders
     C. Skip: Leave everything in place

     Please enter A, B, or C.
     ```

6. **Announce Completion:**
   ```
   🎉 Master Track Complete!

   Master Track: <track_description>
   Subtracks Orchestrated: N/N
   Total Duration: Xh Ym
   Final Checkpoint: <SHA>

   All subtracks have been successfully executed and verified.
   ```

---

## 6.0 ERROR RECOVERY AND RESUME

**PROTOCOL: Handle orchestration failures and enable resume.**

### 6.1 Subtrack Failure

When a subtrack fails during orchestration:

1. **Halt Immediately:**
   - Stop all monitoring
   - Kill any running background subtrack agents
   - Do NOT launch additional subtracks

2. **Display Error Context:**
   - Show failed task details
   - Show error message and location
   - Show phase and progress context

3. **Offer Recovery Options:**
   ```
   Recovery Options:

   A. Retry Failed Subtrack
      Run: /maestro:implement <failed_subtrack_id>
      Then: /maestro:orchestrate <master_track_id> (will resume)

   B. Resume After Manual Fix
      Fix the issue manually, then run:
      /maestro:orchestrate <master_track_id>
      (will skip completed subtracks)

   C. Manual Intervention
      Investigate and fix issues, then resume
   ```

4. **Wait for User:**
   - Do NOT proceed automatically
   - Await user instruction

### 6.2 Resume Capability

When orchestration is restarted:

1. **Read Master Plan:**
   - Parse all tasks and their status
   - Identify tasks marked `[x]` (completed)
   - Identify tasks marked `[~]` (in progress)

2. **Skip Completed Subtracks:**
   - For each orchestration task marked `[x]`:
     - Verify subtrack metadata has status `"completed"`
     - If yes, skip execution
     - If no, re-execute the subtrack

3. **Resume In-Progress Subtracks:**
   - For each orchestration task marked `[~]`:
     - Check if subtrack agent is still running
     - If yes, attach and monitor
     - If no, re-launch subtrack from beginning

4. **Continue Orchestration:**
   - Proceed with remaining tasks
   - Maintain checkpoint state
   - Update progress display

---

## 7.0 PROGRESS TRACKING

**PROTOCOL: Maintain accurate progress tracking throughout orchestration.**

### 7.1 Checkpoint Format

After each subtrack completes, update master plan:

```markdown
- [x] Orchestrate: Execute subtrack '<subtrack_id>'
  Completed: 2025-01-06T02:30:00Z
  Subtrack SHA: a1b2c3d
  Tasks Completed: 18/18
  Duration: 1h 45m
```

### 7.2 Progress Calculation

Calculate overall progress:

```
Total Subtracks: N
Completed Subtracks: C
Overall Progress: (C / N) * 100%

Phase Progress: (completed_tasks_in_phase / total_tasks_in_phase) * 100%
```

### 7.3 Status Updates

Commit plan updates after significant state changes:

- After subtrack completion
- After phase completion
- After verification
- After error recovery

Commit message format: `maestro(plan): <action>`

---

## 8.0 PARALLEL EXECUTION

**PROTOCOL: Execute multiple subtracks simultaneously when safe.**

### 8.1 Parallel Detection

When two orchestration tasks have `**Parallel-With:**` referencing each other:

```markdown
- [ ] Orchestrate: Execute subtrack 'ui_pages_20250105'
  - **Parallel-With:** provider_integrations_20250105

- [ ] Orchestrate: Execute subtrack 'provider_integrations_20250105'
  - **Parallel-With:** ui_pages_20250105
```

### 8.2 Parallel Launch

1. **Verify Both Ready:**
   - Check dependencies for both subtracks
   - Confirm neither is already completed
   - Confirm both have status `"new"` or `"in_progress"`

2. **Launch Simultaneously:**
   - Use Task tool for first subtrack with subagent_type="general-purpose" (run_in_background: true)
   - Immediately use Task tool for second subtrack with subagent_type="general-purpose" (run_in_background: true)
   - Store both task_ids

3. **Monitor Both:**
   - Poll TaskOutput for first subtrack
   - Poll TaskOutput for second subtrack
   - Update progress display for both
   - Wait for BOTH to complete before continuing

4. **Handle Parallel Failure:**
   - If either subtrack fails:
     - Kill the other subtrack agent
     - Halt orchestration
     - Report which subtrack failed and why
     - Report which subtrack was terminated

### 8.3 Parallel Progress Display

```
Phase 2: Core Features (Parallel Execution)
  🔄 ui_pages_20250105 (task 7/18 - 39%)
  🔄 provider_integrations_20250105 (task 9/23 - 39%)
```

---

## 9.0 FINAL NOTES

**CRITICAL REMINDERS:**

1. **Master Track Type:** You are orchestrating, NOT implementing. Delegate all implementation work to sub-tracks.

2. **Agent Delegation - CRITICAL:** Each "Orchestrate" task MUST launch a Claude Code agent using the Task tool with `/maestro:implement <subtrack_id>`. ONLY use Claude Code agents (general-purpose, sonnet-specialist, opus-specialist) because they have Task tool access and can deploy other agents.

3. **Subtrack Agents MUST Deploy Agents:** The subtrack agents you launch MUST follow workflow.md which REQUIRES them to automatically deploy specialized agents (oracle, librarian, macgyver, etc.) based on task complexity. Only Claude Code agents can do this.

4. **Use Aliases from workflow.md:** When workflow.md says "use oracle", understand that this means the review functionality. When you need reviews, use appropriate Claude Code agents with review directives.

5. **Progress Monitoring:** Actively monitor all background agents. Do NOT launch and forget.

6. **Error Handling:** Halt immediately on subtrack failure. Do NOT continue orchestration after errors.

7. **Checkpointing:** Record checkpoints after each subtrack completion. Enable resume capability.

8. **Communication:** Keep user informed with real-time progress updates. Display status clearly.

9. **Documentation:** Reference `maestro/master-track-protocol.md` for detailed protocol information.

10. **Workflow Integration:** For standard tasks (non-orchestration), follow the workflow defined in `maestro/workflow.md`.

---

**END OF ORCHESTRATION PROTOCOL**
