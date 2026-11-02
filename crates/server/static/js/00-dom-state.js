    const statusEl = document.getElementById("status");
    const catalogStatus = document.getElementById("catalog-status");
    const hashEl = document.getElementById("hash");
    const engineEl = document.getElementById("engine");
    const logEl = document.getElementById("log");
    const iframe = document.getElementById("pdf");
    const placeholder = document.getElementById("placeholder");

    function contentHashBust(contentHash) {
      if (!contentHash) return String(Date.now());
      return contentHash.replace(/^sha256:/, "").slice(0, 16);
    }

    /** Content-addressed PDF URL + bust query for embedded PDF viewers. */
    function pdfIframeSrc(pdfUrl, contentHash) {
      const withoutHash = pdfUrl.split("#")[0];
      const base = withoutHash.split("?")[0];
      const bust = contentHashBust(contentHash);
      const view = "view=FitH&navpanes=0&toolbar=1";
      return `${base}?v=${bust}#${view}`;
    }

    function loadPreviewInto(iframeEl, pdfUrl, contentHash) {
      if (!iframeEl || !pdfUrl) return;
      iframeEl.removeAttribute("src");
      iframeEl.src = pdfIframeSrc(pdfUrl, contentHash);
    }

    function loadPreviewPdfIframe(pdfUrl, contentHash) {
      loadPreviewInto(iframe, pdfUrl, contentHash);
    }
    const rebuildBtn = document.getElementById("rebuild");
    const portfolioGroups = document.getElementById("portfolio-groups");
    const catalogCreateParentRow = document.getElementById("catalog-create-parent-row");
    const catalogParentElementSelect = document.getElementById("catalog-parent-element");

    const PORTFOLIO_COLLECTIONS = [
      { id: "contact", label: "Contact", kinds: ["header"] },
      { id: "objective", label: "Objective", kinds: ["objective"] },
      { id: "experience", label: "Professional experience", kinds: ["experience"] },
      { id: "projects", label: "Projects", kinds: ["project"] },
      { id: "education", label: "Education", kinds: ["education"], nestedKind: "course" },
      { id: "certifications", label: "Certifications", kinds: ["certification"] },
      { id: "awards", label: "Awards", kinds: ["award"] },
    ];
    const revisionSelect = document.getElementById("revision-select");
    const payloadEditor = document.getElementById("payload-editor");
    const saveBtn = document.getElementById("save-revision");
    const reloadCatalogBtn = document.getElementById("reload-catalog");
    const tabCatalog = document.getElementById("tab-catalog");
    const tabApps = document.getElementById("tab-apps");
    const panelCatalog = document.getElementById("panel-catalog");
    const panelApps = document.getElementById("panel-apps");
    const appsStatus = document.getElementById("apps-status");
    const appList = document.getElementById("app-list");
    const appListBody = document.getElementById("app-list-body");
    const appListToolbar = document.getElementById("app-list-toolbar");
    const appCreateCloneRow = document.getElementById("app-create-clone-row");
    const appDetail = document.getElementById("app-detail");
    const appMeta = document.getElementById("app-meta");
    const draftEditor = document.getElementById("draft-composition");
    const previewModeWrap = document.getElementById("preview-mode-wrap");
    const previewMode = document.getElementById("preview-mode");
    const draftWarn = document.getElementById("draft-warn");
    const appSearch = document.getElementById("app-search");
    const appPhaseFilter = document.getElementById("app-phase-filter");
    const appWorkflowFilter = document.getElementById("app-workflow-filter");
    const appTagFilter = document.getElementById("app-tag-filter");
    const appPhaseSelect = document.getElementById("app-phase");
    const appWorkflowSelect = document.getElementById("app-workflow");
    const appTagsInput = document.getElementById("app-tags");
    const similarHint = document.getElementById("similar-hint");
    const similarList = document.getElementById("similar-list");
    const cloneFromApp = document.getElementById("clone-from-app");
    const appPostingUrl = document.getElementById("app-posting-url");
    const appPostingText = document.getElementById("app-posting-text");
    const appPostingHintsStatus = document.getElementById("app-posting-hints-status");
    const appPostingHintsList = document.getElementById("app-posting-hints-list");
    const appAnalyzeReport = document.getElementById("app-analyze-report");
    const appRefreshPostingHints = document.getElementById("app-refresh-posting-hints");
    const appCompany = document.getElementById("app-company");
    const appTitle = document.getElementById("app-title");
    const appSource = document.getElementById("app-source");
    const appEmploymentType = document.getElementById("app-employment-type");
    const appWorkMode = document.getElementById("app-work-mode");
    const appLocation = document.getElementById("app-location");
    const appPayUnit = document.getElementById("app-pay-unit");
    const appPayMin = document.getElementById("app-pay-min");
    const appPayMax = document.getElementById("app-pay-max");
    const deleteAppBtn = document.getElementById("delete-app");
    const submitAppBtn = document.getElementById("submit-app");

    let building = false;
    let lifecycleMeta = { phases: [], draft_workflow: "draft", submitted_workflows: [] };
    let selectedElementId = null;
    let elementCreateKind = null;
    let userEditedElementId = false;
    const catalogListBody = document.getElementById("catalog-list-body");
    const catalogElementDetail = document.getElementById("catalog-element-detail");
    const catalogElementIdInput = document.getElementById("catalog-element-id");
    const createElementBtn = document.getElementById("create-element");
    const deleteElementBtn = document.getElementById("delete-element");
    const copyWorkdayFillBtn = document.getElementById("copy-workday-fill");
    const catalogElementMeta = document.getElementById("catalog-element-meta");
    const catalogCreateIdRow = document.getElementById("catalog-create-id-row");
    const catalogRevisionLabel = document.getElementById("catalog-revision-label");
    const catalogCreateHint = document.getElementById("catalog-create-hint");
    const includeInAppWrap = document.getElementById("include-in-app-wrap");
    const includeInOpenApp = document.getElementById("include-in-open-app");
    const newEntryKindSelect = document.getElementById("new-entry-kind");
    let revisionMap = new Map();
    let selectedAppId = null;
    let activeMainTab = "catalog";
    /** Whether the open application has at least one submission (for preview mode). */
    let selectedAppHasSubmission = false;
    let themeInfo = null;
    let previewThemeId = "classic";
    const previewThemeSelect = document.getElementById("preview-theme");
    let catalogByKind = {};
    let elementDetailCache = new Map();
    let selectedCatalogKind = null;
    let selectedElementKind = null;
    let lastCompetencySuggestions = [];
    const skillLinksPanel = document.getElementById("skill-links-panel");
    const skillLinksTool = document.getElementById("skill-links-tool");
    const skillLinksCompetency = document.getElementById("skill-links-competency");
    const skillLinksTag = document.getElementById("skill-links-tag");
    const skillSuggestionsHint = document.getElementById("skill-suggestions-hint");
    const skillCatalogStatus = document.getElementById("skill-catalog-status");
    const skillNodeList = document.getElementById("skill-node-list");
    let selectedPresetId = null;
    let currentFieldSchema = [];
    const payloadForm = document.getElementById("payload-form");
    const catalogJsonMode = document.getElementById("catalog-json-mode");
    const compositionEditor = document.getElementById("composition-editor");
    const tabPresets = document.getElementById("tab-presets");
    const panelPresets = document.getElementById("panel-presets");
    const cloneFromPreset = document.getElementById("clone-from-preset");

    function applicationHasSubmission(app) {
      const subs = app?.submissions;
      if (!Array.isArray(subs) || subs.length === 0) return false;
      return subs.some((s) => s.is_canonical) || subs.length > 0;
    }

    function syncPreviewModeForApp(app) {
      selectedAppHasSubmission = applicationHasSubmission(app);
      const hintEl = document.getElementById("preview-mode-hint");
      if (hintEl) {
        hintEl.textContent = selectedAppHasSubmission
          ? ""
          : " · Last submit after Submit";
      }
      for (const opt of previewMode.options) {
        opt.disabled = false;
      }
      if (!selectedAppHasSubmission && previewMode.value === "submitted") {
        previewMode.value = "draft";
      }
    }

    function ensurePreviewModeAllowed() {
      if (previewMode.value !== "submitted" || selectedAppHasSubmission) {
        return true;
      }
      previewMode.value = "draft";
      statusEl.textContent =
        "No submission yet for this application — previewing current draft.";
      statusEl.classList.remove("error");
      return false;
    }

    function applicationPreviewActive() {
      return resumePreviewActive();
    }

    function selectedPreviewThemeId() {
      return previewThemeSelect?.value || previewThemeId || "classic";
    }

    function syncPreviewThemeFromSelect() {
      previewThemeId = selectedPreviewThemeId();
    }

    function previewBody() {
      const body = { theme_id: selectedPreviewThemeId() };
      if (!resumePreviewActive()) return body;
      if (resumeTarget.type === "application" && resumeTarget.applicationId) {
        body.application_id = resumeTarget.applicationId;
        body.mode = previewMode.value;
        if (previewMode.value === "draft") {
          body.draft_composition = compositionFromEditorDom();
        }
      } else {
        body.mode = "draft";
        body.draft_composition = compositionFromEditorDom();
      }
      return body;
    }

    let previewDebounce = null;
    let previewRebuildQueued = false;
    let previewRequestId = 0;
    /** Skip debounced preview while programmatically filling the composition editor. */
    let compositionEditorSilent = 0;
    let lastPreviewContentHash = null;
    let lastPreviewDownloadFilename = null;
    let lastPreviewPdfUrl = null;

    function invalidatePreviewClientState() {
      previewRequestId += 1;
      lastPreviewContentHash = null;
      lastPreviewDownloadFilename = null;
      lastPreviewPdfUrl = null;
    }

    function applyPreviewSuccess(data, requestId) {
      if (requestId !== previewRequestId) return;
      const themeApplied = data.theme_id || selectedPreviewThemeId();
      const hash = data.content_hash || "";
      const hashShort = hash.replace(/^sha256:/, "").slice(0, 12);
      const src = data.preview_source ? ` (${data.preview_source})` : "";
      statusEl.classList.remove("error");
      statusEl.textContent = data.unchanged
        ? `Preview (${themeApplied}) · ${hashShort}${src}`
        : `Preview rebuilt (${themeApplied}) · ${hashShort}${src}`;
      hashEl.textContent = hash;
      engineEl.textContent = data.pdf_engine ? "PDF engine: " + data.pdf_engine : "";
      if (data.draft_differs_from_submission) {
        draftWarn.classList.remove("hidden");
      } else {
        draftWarn.classList.add("hidden");
      }
      if (data.pdf_url) {
        const samePdf =
          data.pdf_url === lastPreviewPdfUrl && hash === lastPreviewContentHash;
        if (!samePdf) {
          loadPreviewPdfIframe(data.pdf_url, hash);
        }
        lastPreviewPdfUrl = data.pdf_url;
      }
      lastPreviewContentHash = hash;
      lastPreviewDownloadFilename = data.download_filename || null;
    }

    function readCompensationFromDom() {
      const unit = appPayUnit.value || null;
      const minRaw = appPayMin.value.trim();
      const maxRaw = appPayMax.value.trim();
      const min = minRaw ? Number(minRaw) : null;
      const max = maxRaw ? Number(maxRaw) : null;
      if (!unit && min == null && max == null) return {};
      return { unit, min, max };
    }

    function fillCompensationFields(comp) {
      const c = comp || {};
      appPayUnit.value = c.unit || "";
      appPayMin.value = c.min != null && c.min !== "" ? c.min : "";
      appPayMax.value = c.max != null && c.max !== "" ? c.max : "";
    }

    function fillApplicationMetaFields(app) {
      appEmploymentType.value = app?.employment_type || "";
      appWorkMode.value = app?.work_mode || "";
      appLocation.value = app?.location || "";
      fillCompensationFields(app?.compensation);
    }

    function clearApplicationMetaFields() {
      appEmploymentType.value = "";
      appWorkMode.value = "";
      appLocation.value = "";
      fillCompensationFields({});
    }

    function applicationMetaPayload() {
      const employment_type = appEmploymentType.value || null;
      const work_mode = appWorkMode.value || null;
      const location = appLocation.value.trim() || null;
      const compensation = readCompensationFromDom();
      const body = {};
      if (employment_type) body.employment_type = employment_type;
      if (work_mode) body.work_mode = work_mode;
      if (location) body.location = location;
      if (Object.keys(compensation).length) body.compensation = compensation;
      return body;
    }

    function setPreviewPlaceholder(message, visible) {
      placeholder.textContent = message;
      placeholder.classList.toggle("visible", visible);
    }

    /** Single path: read editor → JSON textarea → debounced live preview (unsaved draft). */
    function notifyCompositionChanged() {
      draftEditor.value = JSON.stringify(compositionFromEditorDom(), null, 2);
      if (compositionEditorSilent > 0) return;
      if (previewMode.value !== "draft") return;
      if (!resumePreviewActive()) return;
      clearTimeout(previewDebounce);
      previewDebounce = setTimeout(() => rebuild(), 450);
    }

    async function rebuild() {
      if (building) {
        previewRebuildQueued = true;
        return;
      }
      syncPreviewThemeFromSelect();
      if (!resumePreviewActive()) {
        statusEl.textContent = "";
        hashEl.textContent = "";
        engineEl.textContent = "";
        iframe.removeAttribute("src");
        invalidatePreviewClientState();
        setPreviewPlaceholder("Open the Resume tab to preview PDF.", true);
        return;
      }
      if (resumeTarget.type === "application" && resumeTarget.applicationId) {
        ensurePreviewModeAllowed();
      }
      const requestId = ++previewRequestId;
      building = true;
      statusEl.classList.remove("error");
      logEl.hidden = true;
      draftWarn.classList.add("hidden");
      setPreviewPlaceholder("", false);

      try {
        const res = await fetch("/api/preview", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(previewBody()),
        });
        const data = await res.json();
        if (requestId !== previewRequestId) return;
        if (!data.ok) {
          statusEl.classList.add("error");
          hashEl.textContent = "";
          engineEl.textContent = data.pdf_engine
            ? "PDF engine: " + data.pdf_engine + " (compile failed)"
            : "No PDF engine installed";
          const parts = ["Build failed"];
          if (data.error) parts.push(String(data.error));
          if (data.log_tail) parts.push(String(data.log_tail));
          const detail = parts.join("\n\n");
          const statusCap = 600;
          statusEl.textContent =
            detail.length > statusCap
              ? detail.slice(0, statusCap) + "\n… (full log under Last build)"
              : detail;
          logEl.hidden = false;
          logEl.textContent = detail;
          setPreviewPlaceholder("Fix errors and rebuild", true);
          return;
        }
        if (data.info && !data.pdf_url) {
          statusEl.textContent = data.info;
          statusEl.classList.remove("error");
          hashEl.textContent = "";
          engineEl.textContent = "";
          logEl.hidden = true;
          iframe.removeAttribute("src");
          lastPreviewContentHash = null;
          lastPreviewPdfUrl = null;
          setPreviewPlaceholder(data.info, true);
          return;
        }
        setPreviewPlaceholder("", false);
        applyPreviewSuccess(data, requestId);
      } catch (e) {
        if (requestId !== previewRequestId) return;
        statusEl.textContent = "Request failed: " + e;
        statusEl.classList.add("error");
        setPreviewPlaceholder("Preview unavailable", true);
      } finally {
        building = false;
        if (previewRebuildQueued) {
          previewRebuildQueued = false;
          rebuild();
        }
      }
    }

