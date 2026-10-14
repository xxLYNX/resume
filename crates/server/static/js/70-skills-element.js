    async function loadSkillNodes() {
      await loadSkillCatalogMeta();
      syncHiddenSelectsFromEvidence();
      renderAllEvidenceChips();
    }

    function syncSkillLinksPanelVisibility(kind) {
      const show =
        kind === "experience" ||
        kind === "project" ||
        kind === "education" ||
        kind === "course" ||
        kind === "certification" ||
        kind === "award";
      skillLinksPanel.classList.toggle("hidden", !show);
    }

    function setSkillLinkSelections(rows) {
      setEvidenceFromLinkRows(rows);
    }

    async function loadElementSkillLinks(elementId) {
      if (!elementId) {
        clearEvidenceChips();
        skillSuggestionsHint.classList.add("hidden");
        document.getElementById("skill-comp-suggestions")?.classList.add("hidden");
        return;
      }
      const res = await fetch(
        `/api/catalog/elements/${encodeURIComponent(elementId)}/skill-links`,
      );
      const rows = res.ok ? await res.json() : [];
      setEvidenceFromLinkRows(rows);
      await hydrateSkillNodesFromIds(rows.map((r) => r.skill_id));
      renderAllEvidenceChips();
      await refreshCompetencySuggestions();
    }

    async function refreshCompetencySuggestions() {
      const toolIds = getEvidenceToolIds();
      const box = document.getElementById("skill-comp-suggestions");
      if (!box) return;
      box.innerHTML = "";
      if (toolIds.length === 0) {
        lastCompetencySuggestions = [];
        box.classList.add("hidden");
        skillSuggestionsHint.classList.add("hidden");
        return;
      }
      const q = encodeURIComponent(toolIds.join(","));
      const res = await fetch(`/api/skills/suggestions?tool_ids=${q}`);
      const suggestions = res.ok ? await res.json() : [];
      lastCompetencySuggestions = suggestions;
      if (!suggestions.length) {
        box.classList.add("hidden");
        skillSuggestionsHint.classList.add("hidden");
        return;
      }
      await hydrateSkillNodesFromIds(suggestions.map((s) => s.competency_id));
      box.classList.remove("hidden");
      skillSuggestionsHint.textContent =
        "Suggested competencies (not saved until you click +):";
      skillSuggestionsHint.classList.remove("hidden");
      for (const s of suggestions) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "evidence-suggestion-chip evidence-chip-competency";
        btn.textContent = `+ ${s.label}`;
        btn.addEventListener("click", () => {
          addEvidenceSkillId("competency", s.competency_id);
          cacheSkillNodes([{ id: s.competency_id, kind: "competency", label: s.label }]);
          refreshCompetencySuggestions();
        });
        box.appendChild(btn);
      }
    }

    async function saveElementSkillLinks(elementId) {
      const kind = elementCreateKind || selectedElementKind;
      if (
        kind !== "experience" &&
        kind !== "project" &&
        kind !== "education" &&
        kind !== "course" &&
        kind !== "certification" &&
        kind !== "award"
      ) {
        return;
      }
      const links = collectSkillLinksFromDom();
      const res = await fetch(
        `/api/catalog/elements/${encodeURIComponent(elementId)}/skill-links`,
        {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ links }),
        },
      );
      if (!res.ok) {
        const data = await res.json();
        throw new Error(data.error || "Skill links save failed");
      }
    }

    const saveEvidenceBtn = document.getElementById("save-evidence-tags");
    if (saveEvidenceBtn) {
      saveEvidenceBtn.addEventListener("click", async () => {
        if (!selectedElementId) {
          catalogStatus.textContent = "Select or create an entry first.";
          return;
        }
        try {
          await saveElementSkillLinks(selectedElementId);
          catalogStatus.textContent = "Evidence tags saved.";
          catalogStatus.classList.remove("error");
          await loadCatalog();
          const relOpen = document.getElementById("skill-relationships-details")?.open;
          if (typeof loadSkillRelationships === "function" && relOpen) {
            loadSkillRelationships();
          }
        } catch (e) {
          catalogStatus.textContent = String(e);
          catalogStatus.classList.add("error");
        }
      });
    }

    const skillViewRelBtn = document.getElementById("skill-view-relationships");
    if (skillViewRelBtn) {
      skillViewRelBtn.addEventListener("click", () => {
        if (!selectedElementId) return;
        const details = document.getElementById("skill-relationships-details");
        const queryEl = document.getElementById("skill-rel-query");
        if (queryEl) queryEl.value = selectedElementId;
        if (details) details.open = true;
        if (typeof loadSkillRelationships === "function") loadSkillRelationships();
        catalogListBody.classList.remove("hidden");
        catalogElementDetail.classList.add("hidden");
        loadCatalog();
      });
    }

    async function selectElement(id) {
      elementCreateKind = null;
      selectedElementId = id;
      catalogListBody.classList.add("hidden");
      catalogElementDetail.classList.remove("hidden");
      catalogCreateIdRow.classList.add("hidden");
      catalogCreateParentRow.classList.add("hidden");
      createElementBtn.classList.add("hidden");
      deleteElementBtn.classList.remove("hidden");
      saveBtn.classList.remove("hidden");
      catalogRevisionLabel.classList.remove("hidden");
      catalogCreateHint.classList.add("hidden");
      setPortfolioActive(id);
      const res = await fetch(`/api/catalog/elements/${encodeURIComponent(id)}`);
      const detail = await res.json();
      if (!res.ok) {
        catalogStatus.textContent = detail.error || "Load failed";
        catalogStatus.classList.add("error");
        return;
      }
      catalogElementMeta.textContent = `${detail.kind} · ${detail.id}`;
      selectedCatalogKind = detail.kind;
      selectedElementKind = detail.kind;
      syncWorkdayFillButtonVisibility(detail.kind);
      syncSkillLinksPanelVisibility(detail.kind);
      revisionMap.clear();
      revisionSelect.innerHTML = "";
      for (const rev of detail.revisions) {
        revisionMap.set(rev.id, rev);
        const opt = document.createElement("option");
        opt.value = rev.id;
        opt.textContent = `rev ${rev.rev_number} · ${rev.created_at.slice(0, 19)}`;
        revisionSelect.appendChild(opt);
      }
      revisionSelect.disabled = detail.revisions.length === 0;
      saveBtn.disabled = false;
      payloadEditor.disabled = false;
      if (detail.latest) {
        revisionSelect.value = detail.latest.id;
        setEditorPayload(detail.latest.payload);
        await renderPayloadForm(detail.kind, detail.latest.payload);
        syncCatalogEditorVisibility();
        await loadElementSkillLinks(selectedElementId);
      }
      refreshPortfolioElementResumeHint(selectedElementId, detail.kind);
    }

    function setEditorPayload(payload) {
      payloadEditor.value = JSON.stringify(payload, null, 2);
    }

    revisionSelect.addEventListener("change", async () => {
      const revId = revisionSelect.value;
      const res = await fetch(`/api/catalog/revisions/${encodeURIComponent(revId)}`);
      const rev = await res.json();
      if (res.ok) {
        setEditorPayload(rev.payload);
        if (selectedElementKind) {
          await renderPayloadForm(selectedElementKind, rev.payload);
        }
      }
    });

    saveBtn.addEventListener("click", async () => {
      if (!selectedElementId) return;
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
      saveBtn.disabled = true;
      try {
        const res = await fetch(
          `/api/catalog/elements/${encodeURIComponent(selectedElementId)}/revisions`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ payload }),
          },
        );
        const rev = await res.json();
        if (!res.ok) {
          catalogStatus.textContent = rev.error || "Save failed";
          catalogStatus.classList.add("error");
          return;
        }
        catalogStatus.textContent = `Saved rev ${rev.rev_number}`;
        await selectElement(selectedElementId);
        await loadCatalog();
        await rebuildAfterCatalogSave(selectedElementId);
      } catch (e) {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
      } finally {
        saveBtn.disabled = false;
      }
    });
