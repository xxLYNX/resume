    async function loadSkillNodes() {
      for (const kind of ["tool", "competency"]) {
        const res = await fetch(`/api/skills/nodes?kind=${encodeURIComponent(kind)}`);
        skillNodesCache[kind] = res.ok ? await res.json() : [];
      }
      fillSkillSelect(skillLinksTool, skillNodesCache.tool);
      fillSkillSelect(skillLinksCompetency, skillNodesCache.competency);
      skillNodeList.innerHTML = "";
      const all = [...skillNodesCache.tool, ...skillNodesCache.competency];
      skillCatalogStatus.textContent = `${all.length} skill(s)`;
      for (const node of all.sort((a, b) => a.label.localeCompare(b.label))) {
        const li = document.createElement("li");
        li.textContent = `${node.label} (${node.kind}) · ${node.id}`;
        skillNodeList.appendChild(li);
      }
    }

    function fillSkillSelect(select, nodes) {
      select.innerHTML = "";
      for (const node of nodes) {
        const opt = document.createElement("option");
        opt.value = node.id;
        opt.textContent = node.label;
        select.appendChild(opt);
      }
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
      const byKind = { tool: new Set(), competency: new Set() };
      for (const row of rows || []) {
        if (row.source !== "asserted") continue;
        if (byKind[row.kind]) byKind[row.kind].add(row.skill_id);
      }
      for (const [sel, k] of [
        [skillLinksTool, "tool"],
        [skillLinksCompetency, "competency"],
      ]) {
        for (const opt of sel.options) {
          opt.selected = byKind[k].has(opt.value);
        }
      }
    }

    async function loadElementSkillLinks(elementId) {
      if (!elementId) {
        setSkillLinkSelections([]);
        skillSuggestionsHint.classList.add("hidden");
        return;
      }
      const res = await fetch(
        `/api/catalog/elements/${encodeURIComponent(elementId)}/skill-links`,
      );
      const rows = res.ok ? await res.json() : [];
      setSkillLinkSelections(rows);
      await refreshCompetencySuggestions();
    }

    async function refreshCompetencySuggestions() {
      const toolIds = [...skillLinksTool.selectedOptions].map((o) => o.value);
      const applyBtn = document.getElementById("skill-apply-comp-suggestions");
      if (toolIds.length === 0) {
        lastCompetencySuggestions = [];
        skillSuggestionsHint.classList.add("hidden");
        if (applyBtn) applyBtn.classList.add("hidden");
        return;
      }
      const q = encodeURIComponent(toolIds.join(","));
      const res = await fetch(`/api/skills/suggestions?tool_ids=${q}`);
      const suggestions = res.ok ? await res.json() : [];
      lastCompetencySuggestions = suggestions;
      if (!suggestions.length) {
        skillSuggestionsHint.classList.add("hidden");
        if (applyBtn) applyBtn.classList.add("hidden");
        return;
      }
      skillSuggestionsHint.textContent =
        "Suggested competencies (from selected tools): " +
        suggestions.map((s) => s.label).join(", ");
      skillSuggestionsHint.classList.remove("hidden");
      if (applyBtn) applyBtn.classList.remove("hidden");
    }

    function collectSkillLinksFromDom() {
      const links = [];
      for (const sel of [skillLinksTool, skillLinksCompetency]) {
        for (const opt of sel.selectedOptions) {
          links.push({ skill_id: opt.value, source: "asserted" });
        }
      }
      return links;
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

    skillLinksTool.addEventListener("change", () => refreshCompetencySuggestions());

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
          }
        );
        const rev = await res.json();
        if (!res.ok) {
          catalogStatus.textContent = rev.error || "Save failed";
          catalogStatus.classList.add("error");
          return;
        }
        catalogStatus.textContent = `Saved rev ${rev.rev_number}`;
        try {
          await saveElementSkillLinks(selectedElementId);
          const relOpen = document.getElementById("skill-relationships-details")?.open;
          if (typeof loadSkillRelationships === "function" && relOpen) {
            loadSkillRelationships();
          }
        } catch (e) {
          catalogStatus.textContent = `Saved rev ${rev.rev_number}; skills: ${e.message}`;
        }
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
