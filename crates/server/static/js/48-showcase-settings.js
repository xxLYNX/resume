/** Showcase Resume target, Settings (preferences.toml), and approval UX. */
(() => {
  const showcaseTools = document.getElementById("showcase-tools");
  const showcaseCurrent = document.getElementById("showcase-current");
  const showcaseApproveHint = document.getElementById("showcase-approve-hint");
  const showcaseHistory = document.getElementById("showcase-history");
  const showcaseApproveBtn = document.getElementById("showcase-approve");
  const showcaseApplyDefaultBtn = document.getElementById(
    "showcase-apply-default-theme",
  );
  const openShowcaseBtn = document.getElementById("open-showcase");
  const settingsBtn = document.getElementById("open-settings");
  const settingsDialog = document.getElementById("settings-dialog");
  const settingsStatus = document.getElementById("settings-status");
  const prefCatalog = document.getElementById("pref-catalog-preview");
  const prefShowcase = document.getElementById("pref-showcase");
  const prefApplication = document.getElementById("pref-application");

  const state = {
    updatedAt: null,
    savedCompositionJson: null,
    savedThemeId: null,
    currentPublicationId: null,
    publications: [],
    previewReadyForSaved: false,
    previewSettledRequestId: null,
  };

  function isShowcaseTarget() {
    return (
      typeof resumeTarget !== "undefined" &&
      resumeTarget.type === "showcase"
    );
  }

  function compositionStableJson() {
    try {
      return JSON.stringify(compositionFromEditorDom());
    } catch {
      return null;
    }
  }

  function rules() {
    return window.showcaseApprovalState || null;
  }

  function showcaseHasUnsavedEdits() {
    if (!isShowcaseTarget()) return false;
    const live = compositionStableJson();
    const liveTheme = selectedPreviewThemeId();
    const core = rules();
    if (!core?.showcaseHasUnsavedEdits) {
      return true;
    }
    return core.showcaseHasUnsavedEdits({
      savedComposition: state.savedCompositionJson,
      savedTheme: state.savedThemeId,
      liveComposition: live,
      liveTheme,
    });
  }

  function rulesLoadErrorMessage() {
    return window.showcaseApprovalStateLoadError || null;
  }

  function updateRulesLoadErrorUi() {
    if (!isShowcaseTarget()) return;
    const err = rulesLoadErrorMessage();
    const missing = !rules()?.showcasePreviewEnablesApprove;
    if (err || missing) {
      const msg =
        err ||
        "Showcase approval rules are still loading — approval stays disabled until they are available.";
      if (resumeStatus && err) resumeStatus.textContent = err;
      if (showcaseApproveHint) showcaseApproveHint.textContent = msg;
      markShowcasePreviewStale();
    }
  }

  function showcaseApproveEnabled() {
    return (
      isShowcaseTarget() &&
      !showcaseHasUnsavedEdits() &&
      state.previewReadyForSaved
    );
  }

  function markShowcasePreviewStale() {
    state.previewReadyForSaved = false;
    state.previewSettledRequestId = null;
  }

  function updateShowcasePanel() {
    if (!showcaseTools) return;
    showcaseTools.classList.toggle("hidden", !isShowcaseTarget());
    if (!isShowcaseTarget()) return;
    const current = state.publications.find((p) => p.id === state.currentPublicationId);
    if (showcaseCurrent) {
      if (current) {
        showcaseCurrent.textContent = `Current approved publication: ${current.id.slice(0, 8)}… · ${current.created_at}`;
      } else {
        showcaseCurrent.textContent = "No approved publication yet.";
      }
    }
    updateRulesLoadErrorUi();
    const dirty = showcaseHasUnsavedEdits();
    const rulesMissing = !rules()?.showcasePreviewEnablesApprove;
    const previewPending = !dirty && !rulesMissing && !state.previewReadyForSaved;
    if (showcaseApproveBtn) {
      showcaseApproveBtn.disabled =
        rulesMissing || !showcaseApproveEnabled();
    }
    if (showcaseApplyDefaultBtn) {
      showcaseApplyDefaultBtn.disabled = dirty;
    }
    if (showcaseApproveHint) {
      const rulesErr = rulesLoadErrorMessage();
      if (rulesErr || rulesMissing) {
        showcaseApproveHint.textContent =
          rulesErr ||
          "Showcase approval rules are still loading — approval stays disabled until they are available.";
      } else if (dirty) {
        showcaseApproveHint.textContent =
          "Save the working draft before approving. The PDF preview must match the saved composition and theme, not unsaved editor changes.";
      } else if (previewPending) {
        showcaseApproveHint.textContent =
          "Draft is saved — wait for the PDF preview to finish rebuilding before approving.";
      } else {
        showcaseApproveHint.textContent =
          "Approval publishes the current PDF preview for public distribution. Contact details in the preview will be visible.";
      }
    }
    if (showcaseHistory) {
      showcaseHistory.innerHTML = "";
      const items = [...state.publications].reverse();
      for (const pub of items) {
        const li = document.createElement("li");
        const head = document.createElement("span");
        head.textContent = `${pub.created_at} · ${pub.id.slice(0, 8)}…`;
        if (pub.id === state.currentPublicationId) {
          head.textContent += " (current)";
        }
        li.appendChild(head);
        const row = document.createElement("div");
        row.className = "row";
        const view = document.createElement("a");
        view.href = `/api/showcase/publications/${encodeURIComponent(pub.id)}/resume.pdf`;
        view.target = "_blank";
        view.rel = "noopener";
        view.textContent = "View approved PDF";
        const restore = document.createElement("button");
        restore.type = "button";
        restore.textContent = "Restore pointer";
        restore.addEventListener("click", () => restoreShowcasePublication(pub.id));
        const copy = document.createElement("button");
        copy.type = "button";
        copy.textContent = "Copy composition to working draft";
        copy.addEventListener("click", () => copyPublicationToDraft(pub));
        row.appendChild(view);
        row.appendChild(restore);
        row.appendChild(copy);
        li.appendChild(row);
        showcaseHistory.appendChild(li);
      }
    }
  }

  async function loadShowcaseBundle() {
    const res = await fetch("/api/showcase");
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || "Could not load Showcase");
    state.updatedAt = data.draft.updated_at;
    state.currentPublicationId = data.draft.current_publication_id;
    state.publications = data.publications || [];
    state.savedCompositionJson = JSON.stringify(data.draft.composition);
    state.savedThemeId = data.draft.theme_id || "classic";
    markShowcasePreviewStale();
    if (previewThemeSelect && data.draft.theme_id) {
      previewThemeSelect.value = data.draft.theme_id;
      previewThemeId = data.draft.theme_id;
    }
    return data.draft.composition;
  }

  async function saveShowcaseDraftFromEditor() {
    if (!state.updatedAt) {
      const bundle = await fetch("/api/showcase").then((r) => r.json());
      state.updatedAt = bundle.draft?.updated_at;
    }
    const composition = compositionFromEditorDom();
    const theme_id = selectedPreviewThemeId();
    markShowcasePreviewStale();
    if (typeof invalidatePreviewClientState === "function") {
      invalidatePreviewClientState();
    }
    const res = await fetch("/api/showcase/draft", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        theme_id,
        composition,
        updated_at: state.updatedAt,
      }),
    });
    const draft = await res.json();
    if (!res.ok) throw new Error(draft.error || "Showcase save failed");
    state.updatedAt = draft.updated_at;
    state.savedCompositionJson = JSON.stringify(draft.composition);
    state.savedThemeId = draft.theme_id;
    state.currentPublicationId = draft.current_publication_id;
    updateShowcasePanel();
    if (resumePreviewActive()) await rebuild();
    return draft;
  }

  async function restoreShowcasePublication(publicationId) {
    const res = await fetch("/api/showcase/restore", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ publication_id: publicationId }),
    });
    const draft = await res.json();
    if (!res.ok) throw new Error(draft.error || "Restore failed");
    state.updatedAt = draft.updated_at;
    state.currentPublicationId = draft.current_publication_id;
    await refreshShowcaseBundle();
    updateShowcasePanel();
    if (resumeStatus) resumeStatus.textContent = "Showcase pointer restored.";
  }

  async function copyPublicationToDraft(pub) {
    if (
      !confirm(
        "Copy this publication's composition into the working draft? This does not change approved publications until you save and approve again.",
      )
    ) {
      return;
    }
    compositionEditorSilent += 1;
    try {
      draftEditor.value = JSON.stringify(pub.composition, null, 2);
      await renderCompositionEditor(pub.composition);
      if (previewThemeSelect && pub.theme_id) {
        previewThemeSelect.value = pub.theme_id;
        previewThemeId = pub.theme_id;
      }
      markShowcasePreviewStale();
      notifyCompositionChanged();
    } finally {
      compositionEditorSilent -= 1;
    }
    if (resumeStatus) {
      resumeStatus.textContent =
        "Publication composition loaded in the editor — save the working draft before approving.";
    }
    updateShowcasePanel();
  }

  async function refreshShowcaseBundle() {
    const res = await fetch("/api/showcase");
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || "Could not refresh Showcase");
    state.publications = data.publications || [];
    state.currentPublicationId = data.draft.current_publication_id;
    updateShowcasePanel();
  }

  async function applyShowcaseConfiguredDefaultTheme() {
    if (showcaseHasUnsavedEdits()) {
      if (resumeStatus) {
        resumeStatus.textContent =
          "Save the working draft before applying the configured Showcase default theme.";
      }
      return;
    }
    if (
      !confirm(
        "Apply the Showcase default from Settings to the saved working draft?\n\nThis changes only the saved theme. It does not change composition or approved publications.",
      )
    ) {
      return;
    }
    const res = await fetch("/api/showcase/apply-default-theme", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: state.updatedAt }),
    });
    const body = await res.json();
    if (!res.ok) {
      if (resumeStatus) resumeStatus.textContent = body.error || "Apply default failed";
      if (res.status === 409) await openShowcaseResume();
      return;
    }
    state.updatedAt = body.updated_at;
    state.savedCompositionJson = JSON.stringify(body.composition);
    state.savedThemeId = body.theme_id;
    markShowcasePreviewStale();
    if (previewThemeSelect && body.theme_id) {
      previewThemeSelect.value = body.theme_id;
      previewThemeId = body.theme_id;
    }
    if (resumeStatus) {
      resumeStatus.textContent = `Showcase theme set to configured default (${body.theme_id}).`;
    }
    updateShowcasePanel();
    if (resumePreviewActive()) await rebuild();
  }

  async function approveShowcaseDraft() {
    if (!showcaseApproveEnabled()) {
      if (resumeStatus) {
        resumeStatus.textContent = showcaseHasUnsavedEdits()
          ? "Save the Showcase draft before approving."
          : "Wait for the PDF preview to match the saved draft before approving.";
      }
      return;
    }
    if (
      !confirm(
        "Approve this Showcase for public distribution?\n\nThe PDF in the preview (including contact information) will become the current approved publication.",
      )
    ) {
      return;
    }
    const res = await fetch("/api/showcase/approve", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ updated_at: state.updatedAt }),
    });
    const body = await res.json();
    if (!res.ok) {
      const msg = body.error || "Approval failed";
      if (resumeStatus) resumeStatus.textContent = msg;
      if (res.status === 409) {
        await openShowcaseResume();
      }
      return;
    }
    state.currentPublicationId = body.id;
    state.updatedAt = (await (await fetch("/api/showcase")).json()).draft.updated_at;
    await refreshShowcaseBundle();
    if (resumeStatus) resumeStatus.textContent = `Showcase approved (${body.id.slice(0, 8)}…).`;
  }

  async function openShowcaseResume() {
    activateTab("resume");
    if (typeof setResumeTargetFromValue === "function") {
      await setResumeTargetFromValue("showcase", { reload: true });
    }
  }

  async function fillThemeSelect(select, current) {
    if (!select) return;
    const res = await fetch("/api/themes");
    const ids = res.ok ? await res.json() : [];
    select.innerHTML = "";
    for (const id of ids) {
      const opt = document.createElement("option");
      opt.value = id;
      opt.textContent = id;
      select.appendChild(opt);
    }
    if (current && ids.includes(current)) select.value = current;
  }

  async function loadSettingsDialog() {
    settingsStatus.textContent = "Loading…";
    const res = await fetch("/api/preferences");
    const prefs = await res.json();
    if (!res.ok) {
      settingsStatus.textContent = prefs.error || "Could not load preferences";
      return;
    }
    const themes = prefs.resume?.default_themes || {};
    await fillThemeSelect(prefCatalog, themes.catalog_preview);
    await fillThemeSelect(prefShowcase, themes.showcase);
    await fillThemeSelect(prefApplication, themes.application);
    settingsStatus.textContent = "";
  }

  async function savePreferenceSlot(slot, select) {
    const theme_id = select.value;
    settingsStatus.textContent = `Saving ${slot}…`;
    const res = await fetch("/api/preferences/default-theme", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ slot, theme_id }),
    });
    const prefs = await res.json();
    if (!res.ok) {
      settingsStatus.textContent = prefs.error || "Save failed";
      return;
    }
    settingsStatus.textContent = `Saved ${slot} = ${theme_id}.`;
    window.jobdeskPreferences = prefs;
  }

  openShowcaseBtn?.addEventListener("click", () => {
    openShowcaseResume().catch((e) => {
      if (resumeStatus) resumeStatus.textContent = String(e);
    });
  });

  settingsBtn?.addEventListener("click", () => {
    loadSettingsDialog()
      .then(() => settingsDialog?.showModal())
      .catch((e) => {
        settingsStatus.textContent = String(e);
        settingsDialog?.showModal();
      });
  });

  prefCatalog?.addEventListener("change", () =>
    savePreferenceSlot("catalog_preview", prefCatalog),
  );
  prefShowcase?.addEventListener("change", () =>
    savePreferenceSlot("showcase", prefShowcase),
  );
  prefApplication?.addEventListener("change", () =>
    savePreferenceSlot("application", prefApplication),
  );

  showcaseApplyDefaultBtn?.addEventListener("click", () => {
    applyShowcaseConfiguredDefaultTheme().catch((e) => {
      if (resumeStatus) resumeStatus.textContent = String(e);
    });
  });

  showcaseApproveBtn?.addEventListener("click", () => {
    approveShowcaseDraft().catch((e) => {
      if (resumeStatus) resumeStatus.textContent = String(e);
    });
  });

  function onPreviewSettled(requestId, snapshot) {
    if (!isShowcaseTarget()) return;
    const core = rules();
    const live = compositionStableJson();
    const liveTheme = selectedPreviewThemeId();
    if (!core?.showcasePreviewEnablesApprove) {
      markShowcasePreviewStale();
      updateRulesLoadErrorUi();
      updateShowcasePanel();
      return;
    }
    const enabled = core.showcasePreviewEnablesApprove({
      savedComposition: state.savedCompositionJson,
      savedTheme: state.savedThemeId,
      snapshotComposition: snapshot?.compositionJson ?? null,
      snapshotTheme: snapshot?.themeId ?? null,
      liveComposition: live,
      liveTheme,
      requestId,
      currentRequestId: previewRequestId,
    });
    if (!enabled) {
      markShowcasePreviewStale();
      updateShowcasePanel();
      return;
    }
    state.previewSettledRequestId = requestId;
    state.previewReadyForSaved = true;
    updateShowcasePanel();
  }

  function onPreviewFailed(requestId) {
    if (!isShowcaseTarget()) return;
    if (requestId !== previewRequestId) return;
    markShowcasePreviewStale();
    updateShowcasePanel();
  }

  function onEditorOrThemeChanged() {
    if (!isShowcaseTarget()) return;
    markShowcasePreviewStale();
    updateShowcasePanel();
  }

  window.jobdeskShowcase = {
    isShowcaseTarget,
    loadShowcaseBundle,
    saveShowcaseDraftFromEditor,
    updateShowcasePanel,
    openShowcaseResume,
    onPreviewSettled,
    onPreviewFailed,
    onEditorOrThemeChanged,
    showcaseHasUnsavedEdits,
    showcaseApproveEnabled,
  };

  window.addEventListener("showcase-approval-state-ready", () => {
    updateShowcasePanel();
  });
  window.addEventListener("showcase-approval-state-error", () => {
    updateShowcasePanel();
  });

  fetch("/api/preferences")
    .then((r) => (r.ok ? r.json() : null))
    .then((prefs) => {
      if (prefs) window.jobdeskPreferences = prefs;
    })
    .catch(() => {});
})();