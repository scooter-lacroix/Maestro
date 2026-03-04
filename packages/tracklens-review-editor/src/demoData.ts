/**
 * Demo diff data for TrackLens review editor
 * Sample git diff for testing/development
 */

export interface DiffFile {
  path: string;
  oldPath?: string;
  patch: string;
  additions: number;
  deletions: number;
}

export interface DiffOption {
  id: string;
  label: string;
}

export interface GitContext {
  currentBranch: string;
  defaultBranch: string;
  diffOptions: DiffOption[];
}

export interface DiffData {
  files: DiffFile[];
  gitContext: GitContext;
}

export const DEMO_DIFF: DiffData = {
  files: [
    {
      path: "src/example.ts",
      patch: `diff --git a/src/example.ts b/src/example.ts
index 1234567..abcdefg 100644
--- a/src/example.ts
+++ b/src/example.ts
@@ -1,5 +1,10 @@
-function hello(name: string) {
-  return "Hello " + name;
+function hello(name: string, greeting: string = "Hello") {
+  return greeting + " " + name;
 }

-console.log(hello("World"));
+const result = hello("World", "Hi");
+console.log(result);
+
+export function goodbye(name: string) {
+  return "Goodbye " + name;
+}`,
      additions: 6,
      deletions: 3,
    },
  ],
  gitContext: {
    currentBranch: "feature/add-greeting",
    defaultBranch: "main",
    diffOptions: [
      { id: "working", label: "Working Directory" },
      { id: "staged", label: "Staged Changes" },
    ],
  },
};
