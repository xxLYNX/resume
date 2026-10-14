    let skillNodesById = new Map();
    let skillCatalogTotal = 0;

    function cacheSkillNodes(nodes) {
      for (const node of nodes || []) {
        if (node?.id) skillNodesById.set(node.id, node);
      }
    }

    function skillNodeById(skillId) {
      return skillNodesById.get(skillId) || null;
    }

    async function hydrateSkillNodesFromIds(ids) {
      const missing = [...new Set(ids || [])].filter((id) => id && !skillNodesById.has(id));
      if (!missing.length) return;
      const res = await fetch(
        `/api/skills/nodes?ids=${encodeURIComponent(missing.join(","))}`,
      );
      if (res.ok) cacheSkillNodes(await res.json());
    }

    async function searchSkillNodes(kind, q, limit = 8) {
      const query = String(q || "").trim();
      if (query.length < 2) return [];
      const params = new URLSearchParams({ q: query, limit: String(limit) });
      if (kind) params.set("kind", kind);
      const res = await fetch(`/api/skills/search?${params}`);
      const nodes = res.ok ? await res.json() : [];
      cacheSkillNodes(nodes);
      return nodes;
    }

    async function loadSkillCatalogMeta() {
      const res = await fetch("/api/skills/stats");
      if (!res.ok) {
        skillCatalogStatus.textContent = "Catalog stats unavailable";
        return;
      }
      const data = await res.json();
      const byKind = data.by_kind || {};
      const tool = byKind.tool || 0;
      const comp = byKind.competency || 0;
      const tag = byKind.tag || 0;
      const total = tool + comp + tag;
      skillCatalogTotal = total;
      const banner = document.getElementById("evidence-catalog-empty-banner");
      if (banner) banner.classList.toggle("hidden", total > 0);
      if (!total) {
        skillCatalogStatus.textContent =
          "Catalog empty — run: jobdesk skills import-catalog --onet-dir … --replace --yes";
        skillNodeList.innerHTML =
          "<li>Import O*NET via CLI (see docs/MILESTONE_F_ROLLOUT.md)</li>";
        return;
      }
      skillCatalogStatus.textContent = `${total} catalog node(s): ${tool} tools · ${comp} competencies · ${tag} markers`;
      skillNodeList.innerHTML = "";
      const hint = document.createElement("li");
      hint.textContent = "Use search on an entry to add evidence; full list not loaded in UI.";
      skillNodeList.appendChild(hint);
    }
