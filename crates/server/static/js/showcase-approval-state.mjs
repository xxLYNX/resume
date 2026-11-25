/**
 * Shared Showcase approval / preview readiness rules (browser + node --test).
 * Rust mirror: `domain::showcase_approval`. Contract tests: `domain/tests/fixtures/showcase_approval_cases.json`.
 */

export function showcasePreviewEnablesApprove({
  savedComposition,
  savedTheme,
  snapshotComposition,
  snapshotTheme,
  liveComposition,
  liveTheme,
  requestId,
  currentRequestId,
}) {
  if (requestId !== currentRequestId) return false;
  if (
    savedComposition == null ||
    savedTheme == null ||
    snapshotComposition == null ||
    snapshotTheme == null ||
    liveComposition == null ||
    liveTheme == null
  ) {
    return false;
  }
  if (snapshotComposition !== savedComposition || snapshotTheme !== savedTheme) {
    return false;
  }
  if (liveComposition !== savedComposition || liveTheme !== savedTheme) {
    return false;
  }
  return true;
}

export function showcaseHasUnsavedEdits({
  savedComposition,
  savedTheme,
  liveComposition,
  liveTheme,
}) {
  if (
    savedComposition == null ||
    savedTheme == null ||
    liveComposition == null ||
    liveTheme == null
  ) {
    return true;
  }
  return (
    liveComposition !== savedComposition || liveTheme !== savedTheme
  );
}

export function showcaseRequiresDraftPreviewMode(resumeTargetType, previewModeValue) {
  if (resumeTargetType !== "showcase") return true;
  return previewModeValue === "draft";
}

/** Fail closed when shared rules are missing or not yet loaded. */
export function showcasePreviewApproveEnabled(core, inputs) {
  if (!core?.showcasePreviewEnablesApprove) {
    return false;
  }
  return core.showcasePreviewEnablesApprove(inputs);
}