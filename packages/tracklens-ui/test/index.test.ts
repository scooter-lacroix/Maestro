/**
 * TrackLens UI - Sanity Tests
 *
 * Basic sanity tests to verify exports are available.
 */

import { describe, it, expect } from "bun:test";

// Test that all expected runtime exports are available
describe("TrackLens UI runtime exports", () => {
  it("should export ThemeProvider", () => {
    const { ThemeProvider } = require("../dist/index.js");
    expect(ThemeProvider).toBeDefined();
  });

  it("should export ModeToggle", () => {
    const { ModeToggle } = require("../dist/index.js");
    expect(ModeToggle).toBeDefined();
  });

  it("should export Settings", () => {
    const { Settings } = require("../dist/index.js");
    expect(Settings).toBeDefined();
  });

  it("should export CompletionOverlay", () => {
    const { CompletionOverlay } = require("../dist/index.js");
    expect(CompletionOverlay).toBeDefined();
  });

  it("should export ConfirmDialog", () => {
    const { ConfirmDialog } = require("../dist/index.js");
    expect(ConfirmDialog).toBeDefined();
  });

  it("should export Viewer", () => {
    const { Viewer } = require("../dist/index.js");
    expect(Viewer).toBeDefined();
  });

  it("should export AnnotationPanel", () => {
    const { AnnotationPanel } = require("../dist/index.js");
    expect(AnnotationPanel).toBeDefined();
  });

  it("should export ExportModal", () => {
    const { ExportModal } = require("../dist/index.js");
    expect(ExportModal).toBeDefined();
  });

  it("should export ImportModal", () => {
    const { ImportModal } = require("../dist/index.js");
    expect(ImportModal).toBeDefined();
  });

  it("should export useResizablePanel", () => {
    const { useResizablePanel } = require("../dist/index.js");
    expect(useResizablePanel).toBeDefined();
  });

  it("should export useAutoClose", () => {
    const { useAutoClose } = require("../dist/index.js");
    expect(useAutoClose).toBeDefined();
  });

  it("should export getIdentity", () => {
    const { getIdentity } = require("../dist/index.js");
    expect(getIdentity).toBeDefined();
    expect(typeof getIdentity).toBe("function");
  });

  it("should export storage", () => {
    const { storage } = require("../dist/index.js");
    expect(storage).toBeDefined();
    expect(typeof storage.getItem).toBe("function");
    expect(typeof storage.setItem).toBe("function");
  });

  it("should export AnnotationType enum values", () => {
    const mod = require("../dist/index.js");
    expect(mod.AnnotationType).toBeDefined();
    expect(mod.AnnotationType.COMMENT).toBe("COMMENT");
    expect(mod.AnnotationType.DELETION).toBe("DELETION");
    expect(mod.AnnotationType.INSERTION).toBe("INSERTION");
    expect(mod.AnnotationType.REPLACEMENT).toBe("REPLACEMENT");
  });
});
