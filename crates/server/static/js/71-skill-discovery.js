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
          "No technologies or highlights text to scan on this entry.";
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

    function pickNodeFromCache(kind, value) {
      const v = normalizeSkillToken(value);
      if (!v) return null;
      for (const node of skillNodesById.values()) {
        if (kind && node.kind !== kind) continue;
        if (normalizeSkillToken(node.label) === v || normalizeSkillToken(node.id) === v) {
          return node;
        }
        for (const alias of node.aliases || []) {
          if (normalizeSkillToken(alias) === v) return node;
        }
      }
      return null;
    }

    async function addPickToEvidence(pickInput, kind) {
      let node = pickNodeFromCache(kind, pickInput.value);
      if (!node) {
        const nodes = await searchSkillNodes(kind, pickInput.value, 5);
        node = nodes[0] || null;
      }
      if (!node) {
        catalogStatus.textContent = `No ${kind} matched “${pickInput.value.trim()}”.`;
        return;
      }
      addEvidenceSkillId(kind, node.id);
      pickInput.value = "";
      if (kind === "tool") refreshCompetencySuggestions();
    }

    function wireFilteredPickTypeahead(input, kind) {
      if (!input) return;
      input.removeAttribute("list");
      const menu = document.createElement("div");
      menu.className = "skill-pick-menu hidden";
      menu.setAttribute("role", "listbox");
      input.insertAdjacentElement("afterend", menu);
      let debounce = null;

      function hideMenu() {
        menu.classList.add("hidden");
        menu.innerHTML = "";
      }

      function renderMenu(nodes) {
        menu.innerHTML = "";
        if (!nodes.length) {
          hideMenu();
          return;
        }
        menu.classList.remove("hidden");
        for (const node of nodes) {
          const btn = document.createElement("button");
          btn.type = "button";
          btn.className = "skill-pick-menu-item";
          btn.textContent = node.label;
          btn.addEventListener("mousedown", (e) => e.preventDefault());
          btn.addEventListener("click", () => {
            addEvidenceSkillId(kind, node.id);
            input.value = "";
            hideMenu();
            if (kind === "tool") refreshCompetencySuggestions();
          });
          menu.appendChild(btn);
        }
      }

      input.addEventListener("input", () => {
        clearTimeout(debounce);
        debounce = setTimeout(async () => {
          const q = input.value.trim();
          if (q.length < 2) {
            hideMenu();
            return;
          }
          renderMenu(await searchSkillNodes(kind, q, 8));
        }, 200);
      });
      input.addEventListener("keydown", (e) => {
        if (e.key !== "Enter") return;
        e.preventDefault();
        const first = menu.querySelector(".skill-pick-menu-item");
        if (first && !menu.classList.contains("hidden")) {
          first.click();
          return;
        }
        addPickToEvidence(input, kind);
        hideMenu();
      });
      input.addEventListener("blur", () => setTimeout(hideMenu, 150));
    }

    const skillSuggestFromPayload = document.getElementById("skill-suggest-from-payload");
    const skillCreateUnmatched = document.getElementById("skill-create-unmatched");
    const skillToolPick = document.getElementById("skill-tool-pick");
    const skillCompPick = document.getElementById("skill-comp-pick");
    const skillTagPick = document.getElementById("skill-tag-pick");

    wireFilteredPickTypeahead(skillToolPick, "tool");
    wireFilteredPickTypeahead(skillCompPick, "competency");
    wireFilteredPickTypeahead(skillTagPick, "tag");
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
