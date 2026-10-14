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

    async function findToolIdsForTokens(tokens) {
      const ids = new Set();
      for (const token of tokens) {
        const nodes = await searchSkillNodes("tool", token, 8);
        for (const node of nodes) {
          if (toolNodeMatchesToken(node, token)) ids.add(node.id);
        }
      }
      return ids;
    }

    async function unmatchedTokens(tokens) {
      const out = [];
      for (const token of tokens) {
        const nodes = await searchSkillNodes("tool", token, 4);
        const matched = nodes.some((node) => toolNodeMatchesToken(node, token));
        if (!matched) out.push(token);
      }
      return out;
    }

    function readDetailsFromPayloadForm() {
      const lines = [];
      for (const input of payloadForm.querySelectorAll(
        '.string-list-row[data-field-key="details"] input',
      )) {
        const v = input.value?.trim();
        if (v) lines.push(v);
      }
      if (lines.length) return lines.join("\n");
      const fromJson = readPayloadField("details");
      if (Array.isArray(fromJson)) return fromJson.join("\n");
      return "";
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
      const details = tokenizeEvidenceText(readDetailsFromPayloadForm());
      const hi = tokenizeEvidenceText(readHighlightsFromPayloadForm());
      return uniqueTokens([...details, ...hi]);
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
      addEvidenceSkillIds("tool", ids);
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

    function showToolSuggestPreview(idSet) {
      const box = document.getElementById("skill-suggest-preview");
      if (!box) return;
      box.innerHTML = "";
      if (!idSet.size) {
        box.classList.add("hidden");
        return;
      }
      box.classList.remove("hidden");
      const title = document.createElement("p");
      title.className = "meta";
      title.textContent = "Matched tools from payload (uncheck to skip):";
      box.appendChild(title);
      const list = document.createElement("div");
      for (const id of idSet) {
        const node = skillNodeById(id);
        const label = document.createElement("label");
        label.style.display = "block";
        const cb = document.createElement("input");
        cb.type = "checkbox";
        cb.checked = true;
        cb.dataset.skillId = id;
        label.appendChild(cb);
        label.appendChild(document.createTextNode(" " + (node?.label || id)));
        list.appendChild(label);
      }
      box.appendChild(list);
      const row = document.createElement("div");
      row.className = "row";
      const applyBtn = document.createElement("button");
      applyBtn.type = "button";
      applyBtn.textContent = "Add checked tools";
      applyBtn.addEventListener("click", () => {
        const chosen = new Set();
        for (const cb of list.querySelectorAll('input[type="checkbox"]')) {
          if (cb.checked) chosen.add(cb.dataset.skillId);
        }
        selectToolIds(chosen);
        box.classList.add("hidden");
        refreshCompetencySuggestions();
        catalogStatus.textContent = `Added ${chosen.size} tool(s) from preview. Save evidence tags when ready.`;
      });
      const cancelBtn = document.createElement("button");
      cancelBtn.type = "button";
      cancelBtn.textContent = "Cancel";
      cancelBtn.addEventListener("click", () => box.classList.add("hidden"));
      row.appendChild(applyBtn);
      row.appendChild(cancelBtn);
      box.appendChild(row);
    }

    async function applyToolsFromPayload() {
      const tokens = collectPayloadEvidenceTokens();
      if (!tokens.length) {
        catalogStatus.textContent =
          "No details or highlights text to scan on this entry.";
        syncUnmatchedUi([]);
        return;
      }
      const ids = await findToolIdsForTokens(tokens);
      const unmatched = await unmatchedTokens(tokens);
      syncUnmatchedUi(unmatched);
      showToolSuggestPreview(ids);
      if (ids.size && unmatched.length) {
        catalogStatus.textContent = `${ids.size} tool match(es); review preview. ${unmatched.length} unmatched token(s).`;
      } else if (ids.size) {
        catalogStatus.textContent = `${ids.size} tool match(es); review preview.`;
      } else {
        catalogStatus.textContent = "No catalog tools matched; create markers for unmatched tokens.";
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
        cacheSkillNodes([data]);
        created.push(id);
      }
      await loadSkillCatalogMeta();
      addEvidenceSkillIds("tag", created);
      syncUnmatchedUi([]);
      refreshCompetencySuggestions();
      catalogStatus.textContent = `Created ${created.length} tag(s); add tools manually if needed.`;
      catalogStatus.classList.remove("error");
    }

    const skillSuggestFromPayload = document.getElementById("skill-suggest-from-payload");
    const skillCreateUnmatched = document.getElementById("skill-create-unmatched");
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
