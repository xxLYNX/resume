/** Loads shared Showcase approval rules for classic scripts (non-module). */
(async () => {
  try {
    const mod = await import("/static/js/showcase-approval-state.mjs?v=2");
    window.showcaseApprovalState = mod;
    window.showcaseApprovalStateLoadError = null;
    window.dispatchEvent(new Event("showcase-approval-state-ready"));
  } catch (e) {
    console.error("showcase-approval-state module failed to load", e);
    window.showcaseApprovalState = null;
    window.showcaseApprovalStateLoadError =
      "Showcase approval rules did not load — reload the page (check /static/js/showcase-approval-state.mjs).";
    window.dispatchEvent(new Event("showcase-approval-state-error"));
  }
})();