/**
 * Demo diff data for TrackLens review editor
 * Sample git diff for testing/development
 */

export const DEMO_DIFF = `diff --git a/src/example.ts b/src/example.ts
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
+}
`;
