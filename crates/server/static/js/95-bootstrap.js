    function activateTab(which) {
      activeMainTab = which;
      tabCatalog.classList.toggle("active", which === "catalog");
      tabResume.classList.toggle("active", which === "resume");
      tabApps.classList.toggle("active", which === "apps");
      tabPresets.classList.toggle("active", which === "presets");
      panelCatalog.classList.toggle("hidden", which !== "catalog");
      if (typeof resumeWorkspace !== "undefined" && resumeWorkspace) {
        resumeWorkspace.classList.toggle("hidden", which !== "resume");
      }
      panelApps.classList.toggle("hidden", which !== "apps");
      panelPresets.classList.toggle("hidden", which !== "presets");
      updateMainLayout();
      if (which === "catalog" && selectedElementId && selectedElementKind) {
        refreshPortfolioElementResumeHint(selectedElementId, selectedElementKind);
      }
    }

    tabCatalog.addEventListener("click", () => {
      activateTab("catalog");
      if (!selectedElementId && !isElementCreateMode()) {
        closeElementDetail();
      }
      rebuild();
    });
    tabResume.addEventListener("click", () => {
      activateTab("resume");
      refreshResumeTargetOptions()
        .then(() => loadResumeCompositionForTarget())
        .then(() => rebuild())
        .catch((e) => {
          if (resumeStatus) resumeStatus.textContent = String(e);
        });
    });
    tabApps.addEventListener("click", () => {
      activateTab("apps");
      rebuild();
      loadApplicationStatuses().then(loadApplications);
    });
    tabPresets.addEventListener("click", () => {
      activateTab("presets");
      selectedAppId = null;
      loadPresets();
      rebuild();
    });

    document.getElementById("new-catalog-entry").addEventListener("click", () => {
      openNewElementForm(newEntryKindSelect.value);
    });

    document.getElementById("close-element-detail").addEventListener("click", () => {
      closeElementDetail();
      rebuild();
    });

    copyWorkdayFillBtn.addEventListener("click", () => {
      copyWorkdayFillScript().catch((e) => {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
      });
    });

    deleteElementBtn.addEventListener("click", async () => {
      if (!selectedElementId || isElementCreateMode()) return;
      const id = selectedElementId;
      let usage;
      try {
        const ures = await fetch(`/api/catalog/elements/${encodeURIComponent(id)}/usage`);
        usage = await ures.json();
        if (!ures.ok) {
          catalogStatus.textContent = usage.error || "Could not check usage";
          catalogStatus.classList.add("error");
          return;
        }
      } catch (e) {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
        return;
      }
      if (usage.applications?.length || usage.presets?.length) {
        const parts = [];
        for (const a of usage.applications || []) {
          parts.push(`${a.company} — ${a.title}`);
        }
        for (const p of usage.presets || []) {
          parts.push(`preset: ${p.name}`);
        }
        catalogStatus.textContent =
          "Cannot delete — used in: " + parts.slice(0, 5).join("; ") +
          (parts.length > 5 ? ` (+${parts.length - 5} more)` : "");
        catalogStatus.classList.add("error");
        return;
      }
      if (
        !confirm(
          `Delete “${id}” and all its revisions? This cannot be undone.`
        )
      ) {
        return;
      }
      try {
        const res = await fetch(`/api/catalog/elements/${encodeURIComponent(id)}`, {
          method: "DELETE",
        });
        const data = res.status === 204 ? {} : await res.json();
        if (!res.ok) {
          catalogStatus.textContent = data.error || "Delete failed";
          catalogStatus.classList.add("error");
          return;
        }
        elementDetailCache.delete(id);
        closeElementDetail();
        catalogStatus.textContent = `Deleted ${id}`;
        catalogStatus.classList.remove("error");
        await refreshCatalogByKind();
        await loadCatalog();
        await rebuild();
      } catch (e) {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
      }
    });

    createElementBtn.addEventListener("click", async () => {
      if (!elementCreateKind) return;
      let payload;
      try {
        payload = catalogJsonMode.checked
          ? JSON.parse(payloadEditor.value)
          : payloadFromForm();
      } catch (e) {
        catalogStatus.textContent = "Invalid JSON: " + e.message;
        catalogStatus.classList.add("error");
        return;
      }
      let id = catalogElementIdInput.value.trim();
      if (!id) {
        id = suggestElementIdFromPayload(elementCreateKind, payload);
      }
      const kind = elementCreateKind;
      createElementBtn.disabled = true;
      catalogStatus.textContent = "Creating…";
      try {
        const body = { id, kind, payload };
        if (kind === "course") {
          body.parent_element_id = catalogParentElementSelect.value;
          if (!body.parent_element_id) {
            catalogStatus.textContent = "Choose an education program for this class";
            catalogStatus.classList.add("error");
            createElementBtn.disabled = false;
            return;
          }
        }
        const res = await fetch("/api/catalog/elements", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        });
        const detail = await res.json();
        if (!res.ok) {
          catalogStatus.textContent = detail.error || "Create failed";
          catalogStatus.classList.add("error");
          return;
        }
        elementDetailCache.delete(id);
        elementCreateKind = null;
        await refreshCatalogByKind();
        catalogListBody.classList.remove("hidden");
        await loadCatalog();
        await selectElement(id);
        catalogStatus.textContent = `Created ${id}`;
        const savedId = id;
        const savedKind = kind;
        if (includeInOpenApp.checked && selectedAppId && !isCreateMode()) {
          try {
            await addElementToApplicationDraft(selectedAppId, id, savedKind);
            catalogStatus.textContent = `Created ${id} and added to open application draft`;
          } catch (e) {
            catalogStatus.textContent = `Created ${id}; could not add to application: ${e.message}`;
          }
        } else if (resumePreviewActive()) {
          await refreshCatalogByKind();
          await renderCompositionEditor(compositionFromEditorDom());
          notifyCompositionChanged();
        }
        await rebuildAfterCatalogSave(savedId);
      } catch (e) {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
      } finally {
        createElementBtn.disabled = false;
      }
    });

    async function rebuildAfterCatalogSave(savedElementId) {
      await refreshOpenCompositionRevisionSelects(savedElementId);
      const before = statusEl.textContent;
      await rebuild();
      if (statusEl.classList.contains("error") && savedElementId) {
        catalogStatus.textContent = `Saved ${savedElementId}, but preview failed — see header status / Last build log`;
        catalogStatus.classList.remove("error");
      } else if (!statusEl.classList.contains("error") && before !== statusEl.textContent) {
        catalogStatus.textContent = `Saved ${savedElementId}`;
      }
    }

    reloadCatalogBtn.addEventListener("click", loadCatalog);
    rebuildBtn.addEventListener("click", rebuild);
    previewThemeSelect.addEventListener("change", () => {
      onPreviewThemeChanged().catch((e) => {
        statusEl.textContent = String(e);
        statusEl.classList.add("error");
      });
    });

    loadThemeCatalog()
      .then(loadThemeInfo)
      .then(loadSkillNodes)
      .then(refreshCatalogByKind)
      .then(loadCatalog)
      .then(loadPresets)
      .then(refreshResumeTargetOptions)
      .then(() => loadResumeCompositionForTarget())
      .then(() => {
        updateMainLayout();
        if (resumePreviewActive()) rebuild();
      });
