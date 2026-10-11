
    function appsQueryUrl() {
      const params = new URLSearchParams();
      const q = appSearch.value.trim();
      const phase = appPhaseFilter.value;
      const workflow = appWorkflowFilter.value;
      const tag = appTagFilter.value.trim();
      if (q) params.set("q", q);
      if (phase) params.set("phase", phase);
      if (workflow) params.set("status", workflow);
      if (tag) params.set("tag", tag);
      const qs = params.toString();
      return qs ? `/api/applications?${qs}` : "/api/applications";
    }

    function fillWorkflowSelect(select, includeAll) {
      const current = select.value;
      select.innerHTML = "";
      if (includeAll) {
        const all = document.createElement("option");
        all.value = "";
        all.textContent = "All";
        select.appendChild(all);
      }
      for (const st of lifecycleMeta.submitted_workflows) {
        const opt = document.createElement("option");
        opt.value = st;
        opt.textContent = st;
        select.appendChild(opt);
      }
      if ([...select.options].some((o) => o.value === current)) {
        select.value = current;
      }
    }

    function syncDetailWorkflowOptions(phase, hasSubmission) {
      appPhaseSelect.innerHTML = "";
      for (const p of lifecycleMeta.phases) {
        const opt = document.createElement("option");
        opt.value = p;
        opt.textContent = p;
        if (p === "draft" && hasSubmission) opt.disabled = true;
        if (p === "submitted" && !hasSubmission) opt.disabled = true;
        appPhaseSelect.appendChild(opt);
      }
      appPhaseSelect.value = phase;
      appWorkflowSelect.innerHTML = "";
      if (phase === "draft") {
        const opt = document.createElement("option");
        opt.value = lifecycleMeta.draft_workflow;
        opt.textContent = lifecycleMeta.draft_workflow;
        appWorkflowSelect.appendChild(opt);
        appWorkflowSelect.disabled = true;
      } else {
        appWorkflowSelect.disabled = false;
        for (const st of lifecycleMeta.submitted_workflows) {
          const opt = document.createElement("option");
          opt.value = st;
          opt.textContent = st;
          appWorkflowSelect.appendChild(opt);
        }
      }
    }

    async function loadApplicationStatuses() {
      const res = await fetch("/api/applications/statuses");
      lifecycleMeta = await res.json();
      fillWorkflowSelect(appWorkflowFilter, true);
    }

    async function loadApplications() {
      appsStatus.classList.remove("error");
      appsStatus.textContent = "Loading…";
      appList.innerHTML = "";
      cloneFromApp.innerHTML = '<option value="">Portfolio default</option>';
      try {
        const res = await fetch(appsQueryUrl());
        const data = await res.json();
        if (!res.ok) {
          appsStatus.textContent = data.error || "Failed";
          appsStatus.classList.add("error");
          return;
        }
        appsStatus.textContent = `${data.length} application(s)`;
        for (const app of data) {
          const li = document.createElement("li");
          li.dataset.id = app.id;
          const tagStr = app.tags && app.tags.length ? ` · [${app.tags.join(", ")}]` : "";
          li.textContent = `${app.company} — ${app.title} (${app.phase} · ${app.status})${tagStr}`;
          li.addEventListener("click", () => selectApplication(app.id));
          appList.appendChild(li);
          const opt = document.createElement("option");
          opt.value = app.id;
          opt.textContent = `${app.company} — ${app.title}`;
          cloneFromApp.appendChild(opt);
        }
      } catch (e) {
        appsStatus.textContent = String(e);
        appsStatus.classList.add("error");
      }
    }

    async function loadSimilar(appId) {
      similarList.innerHTML = "";
      similarHint.classList.add("hidden");
      const res = await fetch(`/api/applications/${encodeURIComponent(appId)}/similar?limit=5`);
      const matches = await res.json();
      if (!res.ok || !matches.length) return;
      similarHint.textContent = "Similar applications (quickfill copies draft composition):";
      similarHint.classList.remove("hidden");
      for (const m of matches) {
        const li = document.createElement("li");
        li.textContent = `~${m.overlap_percent}% overlap · ${m.company} — ${m.title}`;
        const btn = document.createElement("button");
        btn.type = "button";
        btn.textContent = "Quickfill";
        btn.addEventListener("click", () => quickfillFrom(m.id));
        li.appendChild(btn);
        similarList.appendChild(li);
      }
    }

    async function quickfillFrom(fromId) {
      if (!selectedAppId) return;
      const res = await fetch(
        `/api/applications/${encodeURIComponent(selectedAppId)}/quickfill`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ from_application_id: fromId }),
        }
      );
      const app = await res.json();
      if (!res.ok) {
        appsStatus.textContent = app.error || "Quickfill failed";
        return;
      }
      draftEditor.value = JSON.stringify(app.draft_composition, null, 2);
      await renderCompositionEditor(app.draft_composition);
      appsStatus.textContent = "Draft copied from similar application";
      await rebuild();
    }

    function isCreateMode() {
      return !appDetail.classList.contains("hidden") && selectedAppId === null;
    }

    function setAppsListVisible(visible) {
      appListBody.classList.toggle("hidden", !visible);
      appListToolbar.classList.toggle("hidden", !visible);
    }

    function closeApplicationDetail() {
      selectedAppId = null;
      appDetail.classList.add("hidden");
      appCreateCloneRow.classList.add("hidden");
      setAppsListVisible(true);
      previewModeWrap.classList.add("hidden");
      for (const li of appList.querySelectorAll("li")) li.classList.remove("active");
      rebuild();
    }

    async function applyCreateCloneSource() {
      if (!isCreateMode()) return;
      let composition;
      try {
        if (cloneFromApp.value) {
          const res = await fetch(
            `/api/applications/${encodeURIComponent(cloneFromApp.value)}`
          );
          const app = await res.json();
          if (!res.ok) throw new Error(app.error || "Load failed");
          composition = app.draft_composition;
        } else if (cloneFromPreset.value) {
          const res = await fetch(`/api/presets/${encodeURIComponent(cloneFromPreset.value)}`);
          const preset = await res.json();
          if (!res.ok) throw new Error(preset.error || "Load failed");
          composition = preset.composition;
        } else {
          composition = await fetchDefaultComposition();
        }
        draftEditor.value = JSON.stringify(composition, null, 2);
        await renderCompositionEditor(composition);
      } catch (e) {
        appsStatus.textContent = String(e);
      }
    }

    async function openNewApplicationForm() {
      selectedAppId = null;
      setAppsListVisible(false);
      appDetail.classList.remove("hidden");
      appCreateCloneRow.classList.remove("hidden");
      previewModeWrap.classList.add("hidden");
      appMeta.textContent = "New application — posting details and resume content below";
      appCompany.value = "";
      appTitle.value = "";
      appPostingUrl.value = "";
      appSource.value = "";
      appTagsInput.value = "";
      clearApplicationMetaFields();
      cloneFromApp.value = "";
      cloneFromPreset.value = "";
      syncDetailWorkflowOptions("draft", false);
      appWorkflowSelect.value = lifecycleMeta.draft_workflow || "draft";
      similarHint.classList.add("hidden");
      similarList.innerHTML = "";
      deleteAppBtn.classList.add("hidden");
      submitAppBtn.classList.add("hidden");
      draftWarn.classList.add("hidden");
      const composition = await fetchDefaultComposition();
      draftEditor.value = JSON.stringify(composition, null, 2);
      await refreshCatalogByKind();
      await renderCompositionEditor(composition);
      await rebuild();
    }

    async function selectApplication(id) {
      selectedAppId = id;
      syncIncludeInAppWrap();
      setAppsListVisible(false);
      appCreateCloneRow.classList.add("hidden");
      deleteAppBtn.classList.remove("hidden");
      submitAppBtn.classList.remove("hidden");
      previewModeWrap.classList.remove("hidden");
      for (const li of appList.querySelectorAll("li")) {
        li.classList.toggle("active", li.dataset.id === id);
      }
      const res = await fetch(`/api/applications/${encodeURIComponent(id)}`);
      const app = await res.json();
      if (!res.ok) {
        appsStatus.textContent = app.error || "Load failed";
        return;
      }
      appDetail.classList.remove("hidden");
      renderAppMeta(app);
      appCompany.value = app.company || "";
      appTitle.value = app.title || "";
      appPostingUrl.value = app.posting_url || "";
      appSource.value = app.source || "";
      fillApplicationMetaFields(app);
      syncDetailWorkflowOptions(app.phase, app.submissions.length > 0);
      syncPreviewModeForApp(app);
      appWorkflowSelect.value = app.status;
      appTagsInput.value = (app.tags || []).join(", ");
      draftEditor.value = JSON.stringify(app.draft_composition, null, 2);
      await refreshCatalogByKind();
      await renderCompositionEditor(app.draft_composition);
      await loadSimilar(id);
      await rebuild();
    }

    function renderAppMeta(app) {
      let base = `${app.title} @ ${app.company} · ${app.submissions.length} submission(s)`;
      if (app.phase === "submitted" && app.submissions.length === 0) {
        base += " · phase/submission mismatch — save as draft or Submit";
      }
      if (app.posting_url) {
        appMeta.innerHTML = `${base} · <a href="${escapeAttr(app.posting_url)}" target="_blank" rel="noopener noreferrer">Open posting</a>`;
      } else {
        appMeta.textContent = base;
      }
    }

    function escapeAttr(s) {
      return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
    }

    document.getElementById("save-app").addEventListener("click", async () => {
      if (!selectedAppId && appDetail.classList.contains("hidden")) return;
      notifyCompositionChanged();
      let draft_composition;
      try {
        draft_composition = compositionFromEditorDom();
      } catch (e) {
        appsStatus.textContent = "Invalid composition: " + e.message;
        return;
      }
      const tags = appTagsInput.value
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean);
      const url = appPostingUrl.value.trim();
      const source = appSource.value.trim();
      const company = appCompany.value.trim();
      const title = appTitle.value.trim();
      if (!company || !title) {
        appsStatus.textContent = "Company and title required";
        return;
      }

      let appId = selectedAppId;
      if (!appId) {
        const createBody = { company, title, tags, ...applicationMetaPayload() };
        if (url) createBody.posting_url = url;
        if (source) createBody.source = source;
        if (cloneFromApp.value) createBody.clone_from_application_id = cloneFromApp.value;
        if (cloneFromPreset.value) createBody.clone_from_preset_id = cloneFromPreset.value;
        const createRes = await fetch("/api/applications", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(createBody),
        });
        const created = await createRes.json();
        if (!createRes.ok) {
          appsStatus.textContent = created.error || "Create failed";
          return;
        }
        appId = created.id;
        selectedAppId = appId;
      }

      const res = await fetch(`/api/applications/${encodeURIComponent(appId)}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          company,
          title,
          phase: appPhaseSelect.value,
          status: appWorkflowSelect.value,
          tags,
          posting_url: url ? url : null,
          source: source ? source : null,
          ...applicationMetaPayload(),
          draft_composition,
        }),
      });
      const app = await res.json();
      if (!res.ok) {
        appsStatus.textContent = app.error || "Save failed";
        return;
      }
      appsStatus.textContent = "Saved";
      appCreateCloneRow.classList.add("hidden");
      deleteAppBtn.classList.remove("hidden");
      submitAppBtn.classList.remove("hidden");
      previewModeWrap.classList.remove("hidden");
      syncPreviewModeForApp(app);
      renderAppMeta(app);
      appCompany.value = app.company || "";
      appTitle.value = app.title || "";
      appPostingUrl.value = app.posting_url || "";
      appSource.value = app.source || "";
      fillApplicationMetaFields(app);
      draftEditor.value = JSON.stringify(app.draft_composition, null, 2);
      await renderCompositionEditor(app.draft_composition);
      await loadApplications();
      for (const li of appList.querySelectorAll("li")) {
        li.classList.toggle("active", li.dataset.id === appId);
      }
      await loadSimilar(appId);
      await rebuild();
    });

    deleteAppBtn.addEventListener("click", async () => {
      if (!selectedAppId) return;
      if (!confirm("Delete this application and its submissions?")) return;
      const id = selectedAppId;
      const res = await fetch(`/api/applications/${encodeURIComponent(id)}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        appsStatus.textContent = data.error || "Delete failed";
        return;
      }
      closeApplicationDetail();
      appsStatus.textContent = "Application deleted";
      await loadApplications();
    });

    document.getElementById("new-app").addEventListener("click", () => {
      openNewApplicationForm();
    });

    document.getElementById("close-app-detail").addEventListener("click", () => {
      closeApplicationDetail();
    });

    cloneFromApp.addEventListener("change", () => {
      cloneFromPreset.value = "";
      applyCreateCloneSource();
    });
    cloneFromPreset.addEventListener("change", () => {
      cloneFromApp.value = "";
      applyCreateCloneSource();
    });

    document.getElementById("reset-draft").addEventListener("click", async () => {
      if (isCreateMode()) {
        cloneFromApp.value = "";
        cloneFromPreset.value = "";
        await applyCreateCloneSource();
        appsStatus.textContent = "Content reset to portfolio default";
        await rebuild();
        return;
      }
      if (!selectedAppId) return;
      const res = await fetch(
        `/api/applications/${encodeURIComponent(selectedAppId)}/reset-draft`,
        { method: "POST" }
      );
      const app = await res.json();
      if (!res.ok) {
        appsStatus.textContent = app.error || "Reset failed";
        return;
      }
      draftEditor.value = JSON.stringify(app.draft_composition, null, 2);
      await renderCompositionEditor(app.draft_composition);
      appsStatus.textContent = "Draft reset to portfolio default";
      await rebuild();
    });

    async function fetchDefaultComposition() {
      const res = await fetch("/api/composition/default");
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "default composition failed");
      return data;
    }

    document.getElementById("save-preset-from-default").addEventListener("click", async () => {
      const name = document.getElementById("new-preset-name").value.trim();
      if (!name) {
        document.getElementById("presets-status").textContent = "Name required";
        return;
      }
      let composition;
      try {
        composition = await fetchDefaultComposition();
      } catch (e) {
        document.getElementById("presets-status").textContent = String(e);
        return;
      }
      const res = await fetch("/api/presets", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, composition }),
      });
      const preset = await res.json();
      if (!res.ok) {
        document.getElementById("presets-status").textContent = preset.error || "Save failed";
        return;
      }
      document.getElementById("new-preset-name").value = "";
      await loadPresets();
      await selectPreset(preset.id);
    });

    document.getElementById("delete-preset").addEventListener("click", async () => {
      if (!selectedPresetId || !confirm("Delete this preset?")) return;
      const res = await fetch(`/api/presets/${encodeURIComponent(selectedPresetId)}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        document.getElementById("presets-status").textContent = "Delete failed";
        return;
      }
      selectedPresetId = null;
      document.getElementById("preset-detail").classList.add("hidden");
      await loadPresets();
    });

    document.getElementById("update-preset-composition").addEventListener("click", async () => {
      if (!selectedPresetId) return;
      let composition;
      try {
        composition = await fetchDefaultComposition();
      } catch (e) {
        document.getElementById("presets-status").textContent = String(e);
        return;
      }
      const res = await fetch(`/api/presets/${encodeURIComponent(selectedPresetId)}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ composition }),
      });
      if (!res.ok) {
        document.getElementById("presets-status").textContent = "Update failed";
        return;
      }
      await loadPresets();
      await selectPreset(selectedPresetId);
    });

    document.getElementById("submit-app").addEventListener("click", async () => {
      if (!selectedAppId) return;
      const res = await fetch(`/api/applications/${encodeURIComponent(selectedAppId)}/submit`, {
        method: "POST",
      });
      const sub = await res.json();
      if (!res.ok) {
        appsStatus.textContent = sub.error || "Submit failed";
        return;
      }
      appsStatus.textContent = `Submitted · ${sub.content_hash.slice(0, 20)}…`;
      await selectApplication(selectedAppId);
      previewMode.value = "submitted";
      await rebuild();
    });

    appPhaseSelect.addEventListener("change", () => {
      const hasSub = appPhaseSelect.querySelector('option[value="draft"]:disabled') != null;
      syncDetailWorkflowOptions(appPhaseSelect.value, hasSub);
    });

    document.getElementById("reload-apps").addEventListener("click", loadApplications);
    document.getElementById("export-ledger-pdf").addEventListener("click", async () => {
      const el = document.getElementById("export-ledger-status");
      el.textContent = "Building PDF…";
      try {
        const res = await fetch("/api/applications/export-ledger", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: "{}",
        });
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || res.statusText);
        el.textContent = "Saved to " + data.path;
      } catch (err) {
        el.textContent = String(err.message || err);
      }
    });
    previewMode.addEventListener("change", () => {
      ensurePreviewModeAllowed();
      rebuild();
    });

