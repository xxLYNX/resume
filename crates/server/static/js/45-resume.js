    /** Resume tab: composition target + preview gating. */
    let resumeTarget = { type: "catalog_default", applicationId: null, presetId: null };

    const tabResume = document.getElementById("tab-resume");
    const panelResume = document.getElementById("panel-resume");
    const mainLayout = document.getElementById("main-layout");
    const panelPreview = document.getElementById("panel-preview");
    const resumeWorkspace = document.getElementById("resume-workspace");
    const resumeTargetSelect = document.getElementById("resume-target-select");
    const resumeStatus = document.getElementById("resume-status");
    const resumePreviewHint = document.getElementById("resume-preview-hint");
    const resumeSaveDraftBtn = document.getElementById("resume-save-draft");
    const downloadPdfBtn = document.getElementById("download-pdf-preview");
    const editTitleLineHighlightsBtn = document.getElementById("edit-title-line-highlights");
    const evidencePortfolioUsage = document.getElementById("evidence-portfolio-usage");
    const evidenceResumeHint = document.getElementById("evidence-resume-hint");

    function resumePreviewActive() {
      return activeMainTab === "resume";
    }

    function resumePreviewTargetKey() {
      const theme = previewThemeSelect?.value || previewThemeId || "classic";
      if (resumeTarget.type === "application" && resumeTarget.applicationId) {
        return `application:${resumeTarget.applicationId}:${previewMode.value}:theme=${theme}`;
      }
      if (resumeTarget.type === "preset" && resumeTarget.presetId) {
        return `preset:${resumeTarget.presetId}:theme=${theme}`;
      }
      return `catalog_default:theme=${theme}`;
    }

    function resetPreviewClientCache() {
      lastPreviewContentHash = null;
      lastPreviewTargetKey = null;
      lastPreviewThemeLoaded = null;
      clearTimeout(previewDebounce);
    }

    function updateResumePreviewHint() {
      if (!resumePreviewHint) return;
      if (!resumePreviewActive()) {
        resumePreviewHint.textContent = "";
        return;
      }
      if (
        resumeTarget.type === "application" &&
        resumeTarget.applicationId &&
        previewMode.value === "submitted"
      ) {
        resumePreviewHint.textContent =
          "Previewing last submit — switch to Current draft to see title-line changes.";
      } else if (resumeTarget.type === "catalog_default") {
        resumePreviewHint.textContent =
          "Preview uses live editor (not saved until you pick an application and Save draft).";
      } else {
        resumePreviewHint.textContent = "Preview uses live editor.";
      }
    }

    function updateMainLayout() {
      if (!mainLayout) return;
      mainLayout.classList.remove(
        "layout-catalog",
        "layout-resume",
        "layout-apps",
        "layout-presets",
      );
      const layout =
        activeMainTab === "resume"
          ? "layout-resume"
          : activeMainTab === "apps"
            ? "layout-apps"
            : activeMainTab === "presets"
              ? "layout-presets"
              : "layout-catalog";
      mainLayout.classList.add(layout);
      if (resumeWorkspace) {
        resumeWorkspace.classList.toggle("hidden", !resumePreviewActive());
      }
      if (previewModeWrap) {
        previewModeWrap.classList.toggle(
          "hidden",
          !resumePreviewActive() || resumeTarget.type !== "application" || !resumeTarget.applicationId,
        );
      }
      if (previewThemeSelect?.closest("label")) {
        previewThemeSelect.closest("label").classList.toggle("hidden", !resumePreviewActive());
      }
      if (rebuildBtn) rebuildBtn.classList.toggle("hidden", !resumePreviewActive());
      if (resumeSaveDraftBtn) {
        resumeSaveDraftBtn.disabled =
          resumeTarget.type === "catalog_default" ||
          (resumeTarget.type === "application" && !resumeTarget.applicationId);
      }
      updateResumePreviewHint();
    }

    async function refreshResumeTargetOptions() {
      if (!resumeTargetSelect) return;
      const prev = resumeTargetSelect.value;
      resumeTargetSelect.innerHTML = "";
      const def = document.createElement("option");
      def.value = "catalog_default";
      def.textContent = "Catalog default (preview only)";
      resumeTargetSelect.appendChild(def);

      const presetGroup = document.createElement("optgroup");
      presetGroup.label = "Presets";
      try {
        const res = await fetch("/api/presets");
        const presets = await res.json();
        if (res.ok) {
          for (const p of presets) {
            const opt = document.createElement("option");
            opt.value = `preset:${p.id}`;
            opt.textContent = p.name;
            presetGroup.appendChild(opt);
          }
        }
      } catch (_) {}
      resumeTargetSelect.appendChild(presetGroup);

      const appGroup = document.createElement("optgroup");
      appGroup.label = "Applications";
      try {
        const res = await fetch("/api/applications");
        const apps = await res.json();
        if (res.ok) {
          for (const a of apps) {
            const opt = document.createElement("option");
            opt.value = `application:${a.id}`;
            opt.textContent = `${a.company} — ${a.title}`;
            appGroup.appendChild(opt);
          }
        }
      } catch (_) {}
      resumeTargetSelect.appendChild(appGroup);

      const want =
        resumeTarget.type === "application" && resumeTarget.applicationId
          ? `application:${resumeTarget.applicationId}`
          : resumeTarget.type === "preset" && resumeTarget.presetId
            ? `preset:${resumeTarget.presetId}`
            : "catalog_default";
      if ([...resumeTargetSelect.options].some((o) => o.value === want)) {
        resumeTargetSelect.value = want;
      } else if (prev && [...resumeTargetSelect.options].some((o) => o.value === prev)) {
        resumeTargetSelect.value = prev;
      }
    }

    function parseResumeTargetValue(value) {
      if (!value || value === "catalog_default") {
        return { type: "catalog_default", applicationId: null, presetId: null };
      }
      if (value.startsWith("application:")) {
        return {
          type: "application",
          applicationId: value.slice("application:".length),
          presetId: null,
        };
      }
      if (value.startsWith("preset:")) {
        return {
          type: "preset",
          applicationId: null,
          presetId: value.slice("preset:".length),
        };
      }
      return { type: "catalog_default", applicationId: null, presetId: null };
    }

    async function loadResumeCompositionForTarget() {
      let composition;
      if (resumeTarget.type === "application" && resumeTarget.applicationId) {
        const res = await fetch(
          `/api/applications/${encodeURIComponent(resumeTarget.applicationId)}`,
        );
        const app = await res.json();
        if (!res.ok) throw new Error(app.error || "Load application failed");
        composition = app.draft_composition;
        selectedAppId = resumeTarget.applicationId;
        syncPreviewModeForApp(app);
      } else if (resumeTarget.type === "preset" && resumeTarget.presetId) {
        const res = await fetch(
          `/api/presets/${encodeURIComponent(resumeTarget.presetId)}`,
        );
        const preset = await res.json();
        if (!res.ok) throw new Error(preset.error || "Load preset failed");
        composition = preset.composition;
      } else {
        composition = await fetchDefaultComposition();
      }
      draftEditor.value = JSON.stringify(composition, null, 2);
      await refreshCatalogByKind();
      await renderCompositionEditor(composition);
      updateResumePreviewHint();
    }

    async function setResumeTargetFromValue(value, { reload = true } = {}) {
      resumeTarget = parseResumeTargetValue(value);
      resetPreviewClientCache();
      updateMainLayout();
      if (reload) {
        try {
          await loadResumeCompositionForTarget();
          if (resumeStatus) resumeStatus.textContent = "";
        } catch (e) {
          if (resumeStatus) resumeStatus.textContent = String(e);
        }
      }
      if (resumePreviewActive()) await rebuild();
    }

    async function syncResumeTargetForApplication(appId, { reload = false } = {}) {
      resumeTarget = {
        type: "application",
        applicationId: appId,
        presetId: null,
      };
      if (resumeTargetSelect) {
        await refreshResumeTargetOptions();
        const v = `application:${appId}`;
        if ([...resumeTargetSelect.options].some((o) => o.value === v)) {
          resumeTargetSelect.value = v;
        }
      }
      updateMainLayout();
      if (reload) await loadResumeCompositionForTarget();
    }

    async function openResumeForApplication(appId) {
      activateTab("resume");
      await syncResumeTargetForApplication(appId, { reload: true });
      await rebuild();
    }

    async function openResumeForProject(elementId) {
      activateTab("resume");
      await refreshResumeTargetOptions();
      let pickedApp = false;
      try {
        const ures = await fetch(
          `/api/catalog/elements/${encodeURIComponent(elementId)}/usage`,
        );
        const usage = ures.ok ? await ures.json() : {};
        const apps = usage.applications || [];
        if (apps.length) {
          await syncResumeTargetForApplication(apps[0].id, { reload: true });
          pickedApp = true;
        } else {
          resumeTarget = { type: "catalog_default", applicationId: null, presetId: null };
          if (resumeTargetSelect) resumeTargetSelect.value = "catalog_default";
          updateMainLayout();
          await loadResumeCompositionForTarget();
          if (resumeStatus) {
            resumeStatus.textContent =
              "Pick an application in Resume target to save title-line highlights.";
          }
        }
      } catch (e) {
        if (resumeStatus) resumeStatus.textContent = String(e);
      }
      await focusProjectRowInComposition(elementId);
      if (pickedApp && resumeStatus) resumeStatus.textContent = "";
      await rebuild();
    }

    async function saveResumeDraftToTarget() {
      let draft_composition;
      try {
        draft_composition = compositionFromEditorDom();
      } catch (e) {
        if (resumeStatus) resumeStatus.textContent = "Invalid composition: " + e.message;
        return;
      }
      if (resumeTarget.type === "application" && resumeTarget.applicationId) {
        const res = await fetch(
          `/api/applications/${encodeURIComponent(resumeTarget.applicationId)}`,
          {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ draft_composition }),
          },
        );
        const app = await res.json();
        if (!res.ok) {
          if (resumeStatus) resumeStatus.textContent = app.error || "Save failed";
          return;
        }
        if (resumeStatus) resumeStatus.textContent = "Application draft saved.";
        draftEditor.value = JSON.stringify(app.draft_composition, null, 2);
        if (
          typeof refreshAppStoredDraftPreview === "function" &&
          selectedAppId === resumeTarget.applicationId
        ) {
          refreshAppStoredDraftPreview(resumeTarget.applicationId);
        }
        return;
      }
      if (resumeTarget.type === "preset" && resumeTarget.presetId) {
        const res = await fetch(
          `/api/presets/${encodeURIComponent(resumeTarget.presetId)}`,
          {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ composition: draft_composition }),
          },
        );
        if (!res.ok) {
          const data = await res.json().catch(() => ({}));
          if (resumeStatus) resumeStatus.textContent = data.error || "Save failed";
          return;
        }
        if (resumeStatus) resumeStatus.textContent = "Preset composition saved.";
        return;
      }
      if (resumeStatus) {
        resumeStatus.textContent =
          "Catalog default is not stored — save as a preset or application draft.";
      }
    }

    async function refreshPortfolioElementResumeHint(elementId, kind) {
      const showProject = kind === "project" && elementId && !isElementCreateMode();
      editTitleLineHighlightsBtn?.classList.toggle("hidden", !showProject);
      if (evidenceResumeHint) {
        evidenceResumeHint.classList.toggle("hidden", !showProject);
        if (showProject) {
          evidenceResumeHint.textContent =
            "PDF subheading tags are chosen on the Resume tab (projects section). Save evidence tags here first.";
        } else {
          evidenceResumeHint.textContent = "";
        }
      }
      if (!evidencePortfolioUsage) return;
      if (!elementId || isElementCreateMode()) {
        evidencePortfolioUsage.classList.add("hidden");
        evidencePortfolioUsage.textContent = "";
        return;
      }
      evidencePortfolioUsage.classList.remove("hidden");
      evidencePortfolioUsage.textContent = "Loading usage…";
      try {
        const linksRes = await fetch(
          `/api/catalog/elements/${encodeURIComponent(elementId)}/skill-links`,
        );
        const links = linksRes.ok ? await linksRes.json() : [];
        const asserted = links.filter((r) => r.source === "asserted");
        const byKind = { tool: 0, competency: 0, tag: 0 };
        for (const row of asserted) {
          byKind[row.kind] = (byKind[row.kind] || 0) + 1;
        }
        const evidencePart = asserted.length
          ? `${byKind.tool || 0} tools · ${byKind.competency || 0} competencies · ${byKind.tag || 0} markers (saved)`
          : "No saved evidence tags on this entry.";
        const ures = await fetch(
          `/api/catalog/elements/${encodeURIComponent(elementId)}/usage`,
        );
        const usage = ures.ok ? await ures.json() : {};
        const parts = [];
        for (const a of usage.applications || []) {
          parts.push(`${a.company} — ${a.title}`);
        }
        for (const p of usage.presets || []) {
          parts.push(`preset: ${p.name}`);
        }
        const usagePart = parts.length
          ? `Used in: ${parts.slice(0, 4).join("; ")}${parts.length > 4 ? ` (+${parts.length - 4})` : ""}`
          : "Not in any saved application or preset draft yet.";
        evidencePortfolioUsage.textContent = `${evidencePart} · ${usagePart}`;
      } catch (e) {
        evidencePortfolioUsage.textContent = String(e);
      }
    }

    function resumeDownloadFilename() {
      if (resumeTargetSelect?.selectedOptions?.[0]?.textContent) {
        const label = resumeTargetSelect.selectedOptions[0].textContent.trim();
        if (label && label !== "Catalog default (preview only)") {
          const safe = label
            .replace(/[^\w\s.-]+/g, "")
            .replace(/\s+/g, "-")
            .slice(0, 80);
          if (safe) return `${safe}.pdf`;
        }
      }
      return "resume.pdf";
    }

    downloadPdfBtn?.addEventListener("click", async () => {
      if (!resumePreviewActive()) return;
      try {
        const res = await fetch(`/api/preview/resume.pdf?t=${Date.now()}`);
        if (!res.ok) {
          if (statusEl) {
            statusEl.textContent = "No PDF yet — rebuild preview first.";
            statusEl.classList.remove("error");
          }
          return;
        }
        const blob = await res.blob();
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = resumeDownloadFilename();
        document.body.appendChild(a);
        a.click();
        a.remove();
        URL.revokeObjectURL(url);
        if (statusEl) statusEl.textContent = "PDF downloaded.";
      } catch (e) {
        if (statusEl) {
          statusEl.textContent = "Download failed: " + e;
          statusEl.classList.add("error");
        }
      }
    });

    if (resumeTargetSelect) {
      resumeTargetSelect.addEventListener("change", () => {
        setResumeTargetFromValue(resumeTargetSelect.value, { reload: true });
      });
    }
    resumeSaveDraftBtn?.addEventListener("click", () => saveResumeDraftToTarget());
    editTitleLineHighlightsBtn?.addEventListener("click", () => {
      if (selectedElementId) openResumeForProject(selectedElementId);
    });
    previewMode?.addEventListener("change", () => {
      updateResumePreviewHint();
      resetPreviewClientCache();
      if (resumePreviewActive()) rebuild();
    });

    document.getElementById("app-edit-resume")?.addEventListener("click", () => {
      if (selectedAppId) openResumeForApplication(selectedAppId);
      else if (isCreateMode()) {
        activateTab("resume");
        refreshResumeTargetOptions();
      }
    });
