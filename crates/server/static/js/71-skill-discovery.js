    let lastUnmatchedTokens = [];

    function normalizeSkillToken(raw) {
      return String(raw)
        .toLowerCase()
        .trim()
        .replace(/[^a-z0-9+#.]+/g, " ");
    }

    function toolNodeMatchesToken(node, token) {
      if (!token) return false;
      const label = normalizeSkillToken(node.label);
      if (label === token || label.includes(token) || token.includes(label)) {
        return true;
      }
      for (const alias of node.aliases || []) {
        const a = normalizeSkillToken(alias);
        if (a === token || a.includes(token) || token.includes(a)) {
          return true;
        }
      }
      return false;
    }

    function tokenizeEvidenceText(text) {
      return String(text || "")
        .split(/[,;\n/|]+/)
        .map((t) => t.trim())
        .filter(Boolean)
        .flatMap((chunk) => {
          const norm = normalizeSkillToken(chunk);
          return norm.length >= 2 ? [norm] : [];
        });
    }

    function uniqueTokens(tokens) {
      return [...new Set(tokens)];
    }

    function findToolIdsForTokens(tokens) {
      const ids = new Set();
      for (const token of tokens) {
        for (const node of skillNodesCache.tool || []) {
          if (toolNodeMatchesToken(node, token)) {
            ids.add(node.id);
          }
        }
      }
      return ids;
    }

    function unmatchedTokens(tokens) {
      const out = [];
      for (const token of tokens) {
        let matched = false;
        for (const node of skillNodesCache.tool || []) {
          if (toolNodeMatchesToken(node, token)) {
            matched = true;
            break;
          }
        }
        if (!matched) out.push(token);
      }
      return out;
    }

    function readTechnologiesFromPayloadForm() {
      const el = payloadForm.querySelector('[data-field-key="technologies"]');
      if (el) return el.value || "";
      return readPayloadField("technologies");
    }

    function readHighlightsFromPayloadForm() {
      const lines = [];
      for (const input of payloadForm.querySelectorAll(
        '.string-list-row[data-field-key="highlights"] input',
      )) {
        const v = input.value?.trim();
        if (v) lines.push(v);
      }
      if (lines.length) return lines.join("\n");
      const fromJson = readPayloadField("highlights");
      if (Array.isArray(fromJson)) return fromJson.join("\n");
      return "";
    }

    function readPayloadField(key) {
      try {
        const payload = catalogJsonMode.checked
          ? JSON.parse(payloadEditor.value || "{}")
          : payloadFromForm();
        return payload?.[key];
      } catch (_) {
        return undefined;
      }
    }

    function collectPayloadEvidenceTokens() {
      const tech = tokenizeEvidenceText(readTechnologiesFromPayloadForm());
      const hi = tokenizeEvidenceText(readHighlightsFromPayloadForm());
      return uniqueTokens([...tech, ...hi]);
    }

    function filterSkillSelect(select, query) {
      const q = normalizeSkillToken(query);
      for (const opt of select.options) {
        const label = normalizeSkillToken(opt.textContent || "");
        const id = normalizeSkillToken(opt.value || "");
        const show = !q || label.includes(q) || id.includes(q);
        opt.hidden = !show;
      }
    }

    function firstVisibleOption(select) {
      for (const opt of select.options) {
        if (!opt.hidden) return opt;
      }
      return null;
    }

    function selectToolIds(ids) {
      for (const opt of skillLinksTool.options) {
        if (ids.has(opt.value)) {
          opt.selected = true;
        }
      }
    }

    function syncUnmatchedUi(tokens) {
      lastUnmatchedTokens = tokens;
      const hint = document.getElementById("skill-unmatched-hint");
      const createBtn = document.getElementById("skill-create-unmatched");
      if (!tokens.length) {
        if (hint) hint.classList.add("hidden");
        if (createBtn) createBtn.classList.add("hidden");
        return;
      }
      if (hint) {
        hint.textContent = `Unmatched tokens (not in catalog): ${tokens.join(", ")}`;
        hint.classList.remove("hidden");
      }
      if (createBtn) createBtn.classList.remove("hidden");
    }

    function applyToolsFromPayload() {
      const tokens = collectPayloadEvidenceTokens();
      if (!tokens.length) {
        catalogStatus.textContent =
          "No technologies or highlights text to scan on this entry.";
        syncUnmatchedUi([]);
        return;
      }
      const ids = findToolIdsForTokens(tokens);
      const unmatched = unmatchedTokens(tokens);
      if (ids.size) selectToolIds(ids);
      syncUnmatchedUi(unmatched);
      refreshCompetencySuggestions();
      if (ids.size && unmatched.length) {
        catalogStatus.textContent = `Selected ${ids.size} tool(s); ${unmatched.length} token(s) need new catalog tools.`;
      } else if (ids.size) {
        catalogStatus.textContent = `Selected ${ids.size} tool(s) from payload.`;
      } else {
        catalogStatus.textContent = "No catalog tools matched; create tools for unmatched tokens.";
      }
    }

    function slugForToolId(token) {
      return String(token)
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-+|-+$/g, "")
        .slice(0, 40);
    }

    async function createTagNodesForUnmatched() {
      if (!lastUnmatchedTokens.length) return;
      const created = [];
      for (const token of lastUnmatchedTokens) {
        const slug = slugForToolId(token);
        if (!slug) continue;
        const id = `tag.${slug}`;
        const label =
          token.length <= 3
            ? token.toUpperCase()
            : token.replace(/\b\w/g, (c) => c.toUpperCase());
        const body = {
          id,
          kind: "tag",
          label,
          aliases: [token, label],
        };
        const res = await fetch("/api/skills/nodes", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(body),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) {
          catalogStatus.textContent = data.error || `Create failed for ${token}`;
          catalogStatus.classList.add("error");
          return;
        }
        created.push(id);
      }
      await loadSkillNodes();
      if (skillLinksTag) {
        for (const id of created) {
          for (const opt of skillLinksTag.options) {
            if (opt.value === id) opt.selected = true;
          }
        }
      }
      syncUnmatchedUi([]);
      refreshCompetencySuggestions();
      catalogStatus.textContent = `Created ${created.length} tag(s); add tools manually if needed.`;
      catalogStatus.classList.remove("error");
    }

    function applyCompetencySuggestionsToSelection() {
      if (!lastCompetencySuggestions.length) return;
      const ids = new Set(lastCompetencySuggestions.map((s) => s.competency_id));
      for (const opt of skillLinksCompetency.options) {
        if (ids.has(opt.value)) {
          opt.selected = true;
        }
      }
      catalogStatus.textContent = "Added suggested competencies to selection.";
    }

    function findNodeByPickValue(kind, value) {
      const v = normalizeSkillToken(value);
      if (!v) return null;
      for (const node of skillNodesCache[kind] || []) {
        if (normalizeSkillToken(node.label) === v || normalizeSkillToken(node.id) === v) {
          return node;
        }
        for (const alias of node.aliases || []) {
          if (normalizeSkillToken(alias) === v) return node;
        }
      }
      for (const node of skillNodesCache[kind] || []) {
        if (
          normalizeSkillToken(node.label).includes(v) ||
          v.includes(normalizeSkillToken(node.label))
        ) {
          return node;
        }
      }
      return null;
    }

    function addPickToSelect(pickInput, selectEl, kind) {
      const node = findNodeByPickValue(kind, pickInput.value);
      if (!node) {
        catalogStatus.textContent = `No ${kind} matched “${pickInput.value.trim()}”.`;
        return;
      }
      for (const opt of selectEl.options) {
        if (opt.value === node.id) {
          opt.selected = true;
          pickInput.value = "";
          if (kind === "tool") refreshCompetencySuggestions();
          return;
        }
      }
    }

    function populateSkillDatalists() {
      const toolDl = document.getElementById("skill-tool-datalist");
      const compDl = document.getElementById("skill-comp-datalist");
      const tagDl = document.getElementById("skill-tag-datalist");
      if (toolDl) {
        toolDl.innerHTML = "";
        for (const node of skillNodesCache.tool || []) {
          const opt = document.createElement("option");
          opt.value = node.label;
          toolDl.appendChild(opt);
        }
      }
      if (compDl) {
        compDl.innerHTML = "";
        for (const node of skillNodesCache.competency || []) {
          const opt = document.createElement("option");
          opt.value = node.label;
          compDl.appendChild(opt);
        }
      }
      if (tagDl) {
        tagDl.innerHTML = "";
        for (const node of skillNodesCache.tag || []) {
          const opt = document.createElement("option");
          opt.value = node.label;
          tagDl.appendChild(opt);
        }
      }
    }

    const _loadSkillNodesOrig = loadSkillNodes;
    loadSkillNodes = async function () {
      await _loadSkillNodesOrig();
      populateSkillDatalists();
    };

    const skillToolFilter = document.getElementById("skill-tool-filter");
    const skillCompFilter = document.getElementById("skill-comp-filter");
    const skillSuggestFromPayload = document.getElementById("skill-suggest-from-payload");
    const skillCreateUnmatched = document.getElementById("skill-create-unmatched");
    const skillApplyCompSuggestions = document.getElementById("skill-apply-comp-suggestions");
    const skillToolPick = document.getElementById("skill-tool-pick");
    const skillCompPick = document.getElementById("skill-comp-pick");
    const skillTagFilter = document.getElementById("skill-tag-filter");
    const skillTagPick = document.getElementById("skill-tag-pick");

    if (skillToolFilter) {
      skillToolFilter.addEventListener("input", () => {
        filterSkillSelect(skillLinksTool, skillToolFilter.value);
      });
      skillToolFilter.addEventListener("keydown", (e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        const opt = firstVisibleOption(skillLinksTool);
        if (opt) {
          opt.selected = true;
          refreshCompetencySuggestions();
        }
      });
    }
    if (skillCompFilter) {
      skillCompFilter.addEventListener("input", () => {
        filterSkillSelect(skillLinksCompetency, skillCompFilter.value);
      });
      skillCompFilter.addEventListener("keydown", (e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        const opt = firstVisibleOption(skillLinksCompetency);
        if (opt) opt.selected = true;
      });
    }
    if (skillToolPick) {
      skillToolPick.addEventListener("change", () =>
        addPickToSelect(skillToolPick, skillLinksTool, "tool"),
      );
      skillToolPick.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          addPickToSelect(skillToolPick, skillLinksTool, "tool");
        }
      });
    }
    if (skillCompPick) {
      skillCompPick.addEventListener("change", () =>
        addPickToSelect(skillCompPick, skillLinksCompetency, "competency"),
      );
      skillCompPick.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          addPickToSelect(skillCompPick, skillLinksCompetency, "competency");
        }
      });
    }
    if (skillTagFilter && skillLinksTag) {
      skillTagFilter.addEventListener("input", () => {
        filterSkillSelect(skillLinksTag, skillTagFilter.value);
      });
      skillTagFilter.addEventListener("keydown", (e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        const opt = firstVisibleOption(skillLinksTag);
        if (opt) opt.selected = true;
      });
    }
    if (skillTagPick && skillLinksTag) {
      skillTagPick.addEventListener("change", () =>
        addPickToSelect(skillTagPick, skillLinksTag, "tag"),
      );
      skillTagPick.addEventListener("keydown", (e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          addPickToSelect(skillTagPick, skillLinksTag, "tag");
        }
      });
    }
    if (skillSuggestFromPayload) {
      skillSuggestFromPayload.addEventListener("click", applyToolsFromPayload);
    }
    if (skillCreateUnmatched) {
      skillCreateUnmatched.addEventListener("click", () => {
        createTagNodesForUnmatched().catch((e) => {
          catalogStatus.textContent = String(e);
          catalogStatus.classList.add("error");
        });
      });
    }
    if (skillApplyCompSuggestions) {
      skillApplyCompSuggestions.addEventListener("click", applyCompetencySuggestionsToSelection);
    }
