import test from "node:test";
import assert from "node:assert/strict";
import {
  showcasePreviewEnablesApprove,
  showcasePreviewApproveEnabled,
  showcaseHasUnsavedEdits,
  showcaseRequiresDraftPreviewMode,
} from "./showcase-approval-state.mjs";

const savedA = JSON.stringify({ sections: [{ section_key: "header", items: [] }] });
const savedTheme = "classic";
const snapshotB = JSON.stringify({
  sections: [{ section_key: "header", items: [{ element_id: "h", revision_id: "r" }] }],
});

test("theme-only unsaved change blocks approval readiness", () => {
  assert.equal(
    showcaseHasUnsavedEdits({
      savedComposition: savedA,
      savedTheme,
      liveComposition: savedA,
      liveTheme: "ats-classic",
    }),
    true,
  );
});

test("stale preview for B does not enable approve after revert to saved A", () => {
  assert.equal(
    showcasePreviewEnablesApprove({
      savedComposition: savedA,
      savedTheme,
      snapshotComposition: snapshotB,
      snapshotTheme: savedTheme,
      liveComposition: savedA,
      liveTheme: savedTheme,
      requestId: 7,
      currentRequestId: 7,
    }),
    false,
  );
});

test("preview for saved A enables approve when editor still matches", () => {
  assert.equal(
    showcasePreviewEnablesApprove({
      savedComposition: savedA,
      savedTheme,
      snapshotComposition: savedA,
      snapshotTheme: savedTheme,
      liveComposition: savedA,
      liveTheme: savedTheme,
      requestId: 3,
      currentRequestId: 3,
    }),
    true,
  );
});

test("superseded request id never enables approve", () => {
  assert.equal(
    showcasePreviewEnablesApprove({
      savedComposition: savedA,
      savedTheme,
      snapshotComposition: savedA,
      snapshotTheme: savedTheme,
      liveComposition: savedA,
      liveTheme: savedTheme,
      requestId: 2,
      currentRequestId: 5,
    }),
    false,
  );
});

test("unavailable or delayed approval rules never enable approve", () => {
  const inputs = {
    savedComposition: savedA,
    savedTheme,
    snapshotComposition: savedA,
    snapshotTheme: savedTheme,
    liveComposition: savedA,
    liveTheme: savedTheme,
    requestId: 1,
    currentRequestId: 1,
  };
  assert.equal(showcasePreviewApproveEnabled(null, inputs), false);
  assert.equal(showcasePreviewApproveEnabled(undefined, inputs), false);
  assert.equal(showcasePreviewApproveEnabled({}, inputs), false);
});

test("showcase target requires draft preview mode", () => {
  assert.equal(showcaseRequiresDraftPreviewMode("showcase", "submitted"), false);
  assert.equal(showcaseRequiresDraftPreviewMode("showcase", "draft"), true);
  assert.equal(showcaseRequiresDraftPreviewMode("application", "submitted"), true);
});