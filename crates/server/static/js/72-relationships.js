    const skillRelDetails = document.getElementById("skill-relationships-details");
    const skillRelStatus = document.getElementById("skill-relationships-status");
    const skillRelBody = document.getElementById("skill-relationships-body");
    const skillRelFilterSkill = document.getElementById("skill-rel-filter-skill");
    const skillRelFilterElement = document.getElementById("skill-rel-filter-element");
    const skillRelFilterSkillKind = document.getElementById("skill-rel-filter-skill-kind");
    const skillRelFilterElementKind = document.getElementById("skill-rel-filter-element-kind");
    const skillRelSkillDatalist = document.getElementById("skill-rel-skill-datalist");

    function populateSkillRelDatalist() {
      if (!skillRelSkillDatalist) return;
      skillRelSkillDatalist.innerHTML = "";
      const all = [...(skillNodesCache.tool || []), ...(skillNodesCache.competency || [])];
      for (const node of all.sort((a, b) => a.label.localeCompare(b.label))) {
        const opt = document.createElement("option");
        opt.value = node.label;
        opt.label = node.id;
        skillRelSkillDatalist.appendChild(opt);
      }
    }

    function resolveSkillIdFromFilterInput(raw) {
      const q = raw.trim();
      if (!q) return "";
      for (const node of [...(skillNodesCache.tool || []), ...(skillNodesCache.competency || [])]) {
        if (node.id === q || node.label === q) return node.id;
      }
      const lower = q.toLowerCase();
      for (const node of [...(skillNodesCache.tool || []), ...(skillNodesCache.competency || [])]) {
        if (node.label.toLowerCase() === lower) return node.id;
        for (const alias of node.aliases || []) {
          if (alias.toLowerCase() === lower) return node.id;
        }
      }
      return q;
    }

    function relationshipsQueryUrl() {
      const params = new URLSearchParams();
      const skillRaw = skillRelFilterSkill?.value.trim();
      if (skillRaw) params.set("skill_id", resolveSkillIdFromFilterInput(skillRaw));
      const el = skillRelFilterElement?.value.trim();
      if (el) params.set("element_id", el);
      const sk = skillRelFilterSkillKind?.value;
      if (sk) params.set("skill_kind", sk);
      const ek = skillRelFilterElementKind?.value;
      if (ek) params.set("element_kind", ek);
      const qs = params.toString();
      return qs ? `/api/skills/relationships?${qs}` : "/api/skills/relationships";
    }

    async function loadSkillRelationships() {
      if (!skillRelBody || !skillRelStatus) return;
      skillRelStatus.textContent = "Loading…";
      skillRelBody.innerHTML = "";
      try {
        const res = await fetch(relationshipsQueryUrl());
        const rows = await res.json();
        if (!res.ok) {
          skillRelStatus.textContent = rows.error || "Load failed";
          return;
        }
        skillRelStatus.textContent = `${rows.length} link(s)`;
        if (!rows.length) return;
        for (const row of rows) {
          const tr = document.createElement("tr");
          tr.dataset.elementId = row.element_id;
          tr.dataset.skillId = row.skill_id;
          tr.innerHTML = `
            <td>${escapeRelText(row.skill_label)}</td>
            <td>${escapeRelText(row.skill_kind)}</td>
            <td class="mono">${escapeRelText(row.element_id)}</td>
            <td>${escapeRelText(row.element_kind)}</td>`;
          tr.addEventListener("click", () => {
            selectElement(row.element_id).catch((e) => {
              catalogStatus.textContent = String(e);
            });
          });
          tr.title = "Open portfolio entry";
          skillRelBody.appendChild(tr);
        }
      } catch (e) {
        skillRelStatus.textContent = String(e);
      }
    }

    function escapeRelText(s) {
      return String(s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/"/g, "&quot;");
    }

    function clearSkillRelFilters() {
      if (skillRelFilterSkill) skillRelFilterSkill.value = "";
      if (skillRelFilterElement) skillRelFilterElement.value = "";
      if (skillRelFilterSkillKind) skillRelFilterSkillKind.value = "";
      if (skillRelFilterElementKind) skillRelFilterElementKind.value = "";
      loadSkillRelationships();
    }

    if (skillRelDetails) {
      skillRelDetails.addEventListener("toggle", () => {
        if (skillRelDetails.open) {
          populateSkillRelDatalist();
          loadSkillRelationships();
        }
      });
    }

    document.getElementById("skill-rel-reload")?.addEventListener("click", loadSkillRelationships);
    document.getElementById("skill-rel-clear")?.addEventListener("click", clearSkillRelFilters);

    const _loadSkillNodesForRel = loadSkillNodes;
    loadSkillNodes = async function () {
      await _loadSkillNodesForRel();
      populateSkillRelDatalist();
      if (skillRelDetails?.open) {
        loadSkillRelationships();
      }
    };
