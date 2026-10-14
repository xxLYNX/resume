    const evidenceChipIds = { tool: new Set(), competency: new Set(), tag: new Set() };

    function syncHiddenSelectsFromEvidence() {
      for (const [sel, kind] of [
        [skillLinksTool, "tool"],
        [skillLinksCompetency, "competency"],
        [skillLinksTag, "tag"],
      ]) {
        if (!sel) continue;
        for (const opt of sel.options) {
          opt.selected = evidenceChipIds[kind].has(opt.value);
        }
      }
    }

    function removeEvidenceSkillId(kind, skillId) {
      if (!evidenceChipIds[kind] || !skillId) return;
      evidenceChipIds[kind].delete(skillId);
      syncHiddenSelectsFromEvidence();
      renderAllEvidenceChips();
      if (kind === "tool") refreshCompetencySuggestions();
    }

    function removeLastEvidenceChip(kind) {
      const ids = [...evidenceChipIds[kind]];
      if (!ids.length) return;
      const sorted = ids.sort((a, b) => {
        const la = skillNodeById(a)?.label || a;
        const lb = skillNodeById(b)?.label || b;
        return la.localeCompare(lb);
      });
      removeEvidenceSkillId(kind, sorted[sorted.length - 1]);
    }

    function renderEvidenceChipRow(kind, container) {
      if (!container) return;
      container.innerHTML = "";
      const ids = [...evidenceChipIds[kind]].sort((a, b) => {
        const la = skillNodeById(a)?.label || a;
        const lb = skillNodeById(b)?.label || b;
        return la.localeCompare(lb);
      });
      for (const id of ids) {
        const node = skillNodeById(id);
        const chip = document.createElement("span");
        chip.className = `evidence-chip evidence-chip-${kind}`;
        chip.title = id;
        const label = document.createElement("span");
        label.textContent = node?.label || id;
        const removeBtn = document.createElement("button");
        removeBtn.type = "button";
        removeBtn.className = "evidence-chip-remove";
        removeBtn.setAttribute("aria-label", `Remove ${node?.label || id}`);
        removeBtn.textContent = "×";
        removeBtn.addEventListener("mousedown", (e) => e.preventDefault());
        removeBtn.addEventListener("click", (e) => {
          e.stopPropagation();
          removeEvidenceSkillId(kind, id);
        });
        chip.appendChild(label);
        chip.appendChild(removeBtn);
        container.appendChild(chip);
      }
    }

    function renderAllEvidenceChips() {
      renderEvidenceChipRow("tool", document.getElementById("evidence-chips-tool"));
      renderEvidenceChipRow("competency", document.getElementById("evidence-chips-competency"));
      renderEvidenceChipRow("tag", document.getElementById("evidence-chips-marker"));
      const meta = document.getElementById("evidence-chips-meta");
      if (meta && selectedElementId) {
        const n =
          evidenceChipIds.tool.size +
          evidenceChipIds.competency.size +
          evidenceChipIds.tag.size;
        meta.textContent = n
          ? `${n} evidence tag(s) on this entry (unsaved until you click Save evidence tags)`
          : "No evidence tags on this entry yet";
      }
    }

    function setEvidenceFromLinkRows(rows) {
      evidenceChipIds.tool.clear();
      evidenceChipIds.competency.clear();
      evidenceChipIds.tag.clear();
      for (const row of rows || []) {
        if (row.source !== "asserted") continue;
        if (evidenceChipIds[row.kind]) evidenceChipIds[row.kind].add(row.skill_id);
      }
      syncHiddenSelectsFromEvidence();
      renderAllEvidenceChips();
    }

    function clearEvidenceChips() {
      setEvidenceFromLinkRows([]);
    }

    function addEvidenceSkillId(kind, skillId) {
      if (!evidenceChipIds[kind] || !skillId) return;
      evidenceChipIds[kind].add(skillId);
      syncHiddenSelectsFromEvidence();
      renderAllEvidenceChips();
    }

    function addEvidenceSkillIds(kind, ids) {
      for (const id of ids) addEvidenceSkillId(kind, id);
    }

    function getEvidenceToolIds() {
      return [...evidenceChipIds.tool];
    }

    function collectSkillLinksFromDom() {
      const links = [];
      for (const kind of ["tool", "competency", "tag"]) {
        for (const skill_id of evidenceChipIds[kind]) {
          links.push({ skill_id, source: "asserted" });
        }
      }
      return links;
    }
