    const skillRelDetails = document.getElementById("skill-relationships-details");
    const skillRelStatus = document.getElementById("skill-relationships-status");
    const skillRelBody = document.getElementById("skill-relationships-body");
    const skillRelQuery = document.getElementById("skill-rel-query");
    const skillRelFilterSkill = document.getElementById("skill-rel-filter-skill");
    const skillRelFilterElement = document.getElementById("skill-rel-filter-element");
    const skillRelFilterSkillKind = document.getElementById("skill-rel-filter-skill-kind");
    const skillRelFilterElementKind = document.getElementById("skill-rel-filter-element-kind");

    async function resolveSkillIdFromFilterInput(raw) {
      const q = raw.trim();
      if (!q) return "";
      for (const node of skillNodesById.values()) {
        if (node.id === q || node.label === q) return node.id;
      }
      const lower = q.toLowerCase();
      for (const node of skillNodesById.values()) {
        if (node.label.toLowerCase() === lower) return node.id;
        for (const alias of node.aliases || []) {
          if (alias.toLowerCase() === lower) return node.id;
        }
      }
      const nodes = await searchSkillNodes(null, q, 5);
      if (nodes[0]) return nodes[0].id;
      return q;
    }

    function syncAdvancedFiltersFromQueryTokens(tokens) {
      if (!skillRelFilterSkillKind || !skillRelFilterElementKind) return;
      for (const t of tokens) {
        const m = t.match(/^kind:(tool|competency|tag)$/i);
        if (m) skillRelFilterSkillKind.value = m[1].toLowerCase();
        const e = t.match(/^entry:(experience|project|education|course|certification|award)$/i);
        if (e) skillRelFilterElementKind.value = e[1].toLowerCase();
      }
    }

    async function buildRelationshipParamsFromQuery() {
      const params = new URLSearchParams();
      const raw = skillRelQuery?.value.trim() || "";
      const tokens = raw
        .split(/[,;\n]+/)
        .map((t) => t.trim())
        .filter(Boolean);

      syncAdvancedFiltersFromQueryTokens(tokens);

      const advSkill = skillRelFilterSkill?.value.trim();
      const advEl = skillRelFilterElement?.value.trim();
      const advSk = skillRelFilterSkillKind?.value;
      const advEk = skillRelFilterElementKind?.value;

      if (advSkill) params.set("skill_id", await resolveSkillIdFromFilterInput(advSkill));
      if (advEl) params.set("element_id", advEl);
      if (advSk) params.set("skill_kind", advSk);
      if (advEk) params.set("element_kind", advEk);

      if (params.toString()) return params;

      for (const token of tokens) {
        if (/^kind:/i.test(token) || /^entry:/i.test(token)) continue;
        if (/^(experience|project|education|course|certification|award)\./i.test(token)) {
          params.set("element_id", token);
          continue;
        }
        if (!params.has("skill_id")) {
          params.set("skill_id", await resolveSkillIdFromFilterInput(token));
        }
      }
      return params;
    }

    async function relationshipsQueryUrl() {
      const params = await buildRelationshipParamsFromQuery();
      const qs = params.toString();
      return qs ? `/api/skills/relationships?${qs}` : null;
    }

    async function loadSkillRelationships() {
      if (!skillRelBody || !skillRelStatus) return;
      const url = await relationshipsQueryUrl();
      if (!url) {
        skillRelStatus.textContent = "Enter a query (or advanced filters) and Search.";
        skillRelBody.innerHTML = "";
        return;
      }
      skillRelStatus.textContent = "Loading…";
      skillRelBody.innerHTML = "";
      try {
        const res = await fetch(url);
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
      if (skillRelQuery) skillRelQuery.value = "";
      if (skillRelFilterSkill) skillRelFilterSkill.value = "";
      if (skillRelFilterElement) skillRelFilterElement.value = "";
      if (skillRelFilterSkillKind) skillRelFilterSkillKind.value = "";
      if (skillRelFilterElementKind) skillRelFilterElementKind.value = "";
      skillRelBody.innerHTML = "";
      if (skillRelStatus) {
        skillRelStatus.textContent = "Enter a query and Search.";
      }
    }

    if (skillRelDetails) {
      skillRelDetails.addEventListener("toggle", () => {
        if (skillRelDetails.open && skillRelStatus && !skillRelBody.children.length) {
          skillRelStatus.textContent = "Enter a query and Search.";
        }
      });
    }

    document.getElementById("skill-rel-search")?.addEventListener("click", loadSkillRelationships);
    document.getElementById("skill-rel-clear")?.addEventListener("click", clearSkillRelFilters);
    skillRelQuery?.addEventListener("keydown", (e) => {
      if (e.key === "Enter") {
        e.preventDefault();
        loadSkillRelationships();
      }
    });

    const _loadSkillNodesForRel = loadSkillNodes;
    loadSkillNodes = async function () {
      await _loadSkillNodesForRel();
    };
