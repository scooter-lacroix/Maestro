---
description: Displays the current progress of the project
argument-hint: [no arguments]
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
  - AskUserQuestion
model: haiku
---

## 1.0 SYSTEM DIRECTIVE
You are an AI agent. Your primary function is to provide a status overview of the current tracks file. This involves reading the `maestro/tracks.md` file, parsing its content, and summarizing the progress of tasks.

**CRITICAL:** Before proceeding, you should start by checking if the project has been properly set up.
1.  **Verify Tracks File:** Check if the file `maestro/tracks.md` exists. If it does not, HALT execution and instruct the user: "The project has not been set up or maestro/tracks.md has been corrupted. Please run `/maestro:setup` to set up the plan, or restore maestro/tracks.md."
2.  **Verify Track Exists:** Check if the file `maestro/tracks.md` is not empty. If it is empty, HALT execution and instruct the user: "The project has not been set up or maestro/tracks.md has been corrupted. Please run `/maestro:setup` to set up the plan, or restore maestro/tracks.md."

CRITICAL: You must validate the success of every tool call. If any tool call fails, you MUST halt the current operation immediately, announce the failure to the user, and await further instructions.

---


## 1.1 SETUP CHECK
**PROTOCOL: Verify that the Maestro environment is properly set up.**

1.  **Check for Required Files:** You MUST verify the existence of the following files in the `maestro` directory:
    -   `maestro/tech-stack.md`
    -   `maestro/workflow.md`
    -   `maestro/product.md`

2.  **Handle Missing Files:**
    -   If ANY of these files are missing, you MUST halt the operation immediately.
    -   Announce: "Maestro is not set up. Please run `/maestro:setup` to set up the environment."
    -   Do NOT proceed to Status Overview Protocol.

---

## 2.0 STATUS OVERVIEW PROTOCOL
**PROTOCOL: Follow this sequence to provide a status overview.**

### 2.1 Read Project Plan
1.  **Locate and Read:** Read the content of the `maestro/tracks.md` file.
2.  **Locate and Read:** List the tracks using shell command `ls maestro/tracks`. For each of the tracks, read the corresponding `maestro/<track_id>/plan.md` file.

### 2.2 Parse and Summarize Plan
1.  **Parse Content:**
    -   Identify major project phases/sections (e.g., top-level markdown headings).
    -   Identify individual tasks and their current status (e.g., bullet points under headings, looking for keywords like "COMPLETED", "IN PROGRESS", "PENDING").
2.  **Generate Summary:** Create a concise summary of the project's overall progress. This should include:
    -   The total number of major phases.
    -   The total number of tasks.
    -   The number of tasks completed, in progress, and pending.

### 2.3 Present Status Overview
1.  **Output Summary:** Present the generated summary to the user in a clear, readable format. The status report must include:
    -   **Current Date/Time:** The current timestamp.
    -   **Project Status:** A high-level summary of progress (e.g., "On Track", "Behind Schedule", "Blocked").
    -   **Current Phase and Task:** The specific phase and task currently marked as "IN PROGRESS".
    -   **Next Action Needed:** The next task listed as "PENDING".
    -   **Blockers:** Any items explicitly marked as blockers in the plan.
    -   **Phases (total):** The total number of major phases.
    -   **Tasks (total):** The total number of tasks.
    -   **Progress:** The overall progress of the plan, presented as tasks_completed/tasks_total (percentage_completed%).
    -   **Memory Context:** Last stored memory timestamp and summary

    **Memory Context Retrieval Protocol:**

    a. Import the memory management modules:
       ```python
       from maestro.memory.database.models import get_session, Memory, MaestroProject
       from maestro.core.tracks.models import TrackManager
       from maestro.core.tracks.repository import TrackRepository
       import os
       ```

    b. Initialize the memory system:
       ```python
       db_session = get_session()
       project_path = os.getcwd()
       track_manager = TrackManager(db_session, project_path)
       track_repository = TrackRepository("maestro/tracks")
       ```

    c. Get recent memories for the project:
       ```python
       # Get recent memories across all tracks
       project_id = track_manager.get_or_create_project()
       recent_memories = db_session.query(Memory).filter(
           Memory.project_id == project_id
       ).order_by(Memory.created_at.desc()).limit(10).all()

       # Format memory context
       memory_context = []
       for memory in recent_memories:
           memory_context.append({
               "content": memory.content[:200] + "..." if len(memory.content) > 200 else memory.content,
               "category": memory.category,
               "importance": memory.importance,
               "created_at": memory.created_at.isoformat() if memory.created_at else None,
               "track_id": memory.track_id,
           })
       ```

    d. Get track-specific summaries:
       ```python
       tracks = track_repository.list_tracks()
       track_summaries = []
       for track in tracks:
           if track.get("maestro_track_id"):
               summary = track_manager.get_track_summary(track["track_id"])
               if summary.get("found"):
                   track_summaries.append({
                       "track_id": track["track_id"],
                       "title": summary.get("title"),
                       "status": summary.get("status"),
                       "progress": summary.get("progress", 0),
                   })
       ```

    e. Include memory context in status report:
       ```
       **Memory Context:**
       - Total memories stored: {len(memory_context)}
       - Most recent: {most_recent_memory['created_at']} - {most_recent_memory['summary']}

       **Track Status from Memory:**
       {for track in track_summaries}
       - {track['track_id']}: {track['status']} ({track['progress']:.0f}% complete)
       {endfor}
       ```

    f. Check for pending handoffs:
       ```python
       from maestro.memory.coordination.handoffs import HandoffHandler
       handler = HandoffHandler(db_session)
       pending_handoffs = handler.get_pickable_handoffs(project_id=project_id, limit=5)

       # Include in status if any exist
       if pending_handoffs:
           print("**Pending Handoffs:**")
           for handoff in pending_handoffs:
               print(f"- {handoff.handoff_id}: {handoff.title} ({handoff.status})")
       ```
