    function selectEvidenceTagFromNode(kind, node, input, menu, hint) {
      if (!node?.id) return;
      addEvidenceSkillId(kind, node.id);
      cacheSkillNodes([node]);
      if (input) input.value = "";
      closeEvidenceTagMenu(menu);
      if (hint) hint.textContent = "";
      if (kind === "tool") refreshCompetencySuggestions();
    }

    function closeEvidenceTagMenu(menu) {
      if (!menu) return;
      menu.classList.add("hidden");
      menu.innerHTML = "";
    }

    function setMenuStatus(menu, text) {
      if (!menu) return;
      menu.innerHTML = "";
      menu.classList.remove("hidden");
      const p = document.createElement("div");
      p.className = "evidence-tag-menu-status";
      p.textContent = text;
      menu.appendChild(p);
    }

    async function addPickToEvidence(pickInput, kind) {
      const q = pickInput.value.trim();
      if (!q) return;
      let node = pickNodeFromCache(kind, q);
      if (!node) {
        const nodes = await searchSkillNodes(kind, q, 5);
        node = nodes[0] || null;
      }
      if (!node) {
        catalogStatus.textContent = `No ${kind} matched “${q}”.`;
        return;
      }
      selectEvidenceTagFromNode(kind, node, pickInput, null, null);
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

    function wireEvidenceTagField({ kind, input, menu, hint, box }) {
      if (!input || !menu) return;
      let debounce = null;
      let activeIndex = -1;
      let lastNodes = [];

      function setHint(text) {
        if (hint) hint.textContent = text || "";
      }

      function renderMenuItems(nodes) {
        lastNodes = nodes;
        activeIndex = nodes.length ? 0 : -1;
        menu.innerHTML = "";
        if (!nodes.length) {
          if (skillCatalogTotal === 0) {
            setMenuStatus(menu, "Catalog empty — import O*NET via CLI first.");
          } else {
            setMenuStatus(menu, "No catalog matches.");
          }
          return;
        }
        menu.classList.remove("hidden");
        nodes.forEach((node, i) => {
          const btn = document.createElement("button");
          btn.type = "button";
          btn.className = "evidence-tag-menu-item" + (i === activeIndex ? " is-active" : "");
          btn.textContent = node.label;
          btn.dataset.index = String(i);
          btn.addEventListener("mousedown", (e) => e.preventDefault());
          btn.addEventListener("click", () => {
            selectEvidenceTagFromNode(kind, node, input, menu, hint);
          });
          menu.appendChild(btn);
        });
      }

      function highlightActive() {
        const items = menu.querySelectorAll(".evidence-tag-menu-item");
        items.forEach((el, i) => el.classList.toggle("is-active", i === activeIndex));
        const active = items[activeIndex];
        if (active) active.scrollIntoView({ block: "nearest" });
      }

      async function runSearch() {
        const q = input.value.trim();
        if (q.length < 2) {
          closeEvidenceTagMenu(menu);
          setHint(q.length ? "Type 2+ letters to search catalog" : "");
          return;
        }
        setHint("Searching…");
        setMenuStatus(menu, "Searching…");
        try {
          const nodes = await searchSkillNodes(kind, q, 8);
          setHint("");
          renderMenuItems(nodes);
        } catch (e) {
          closeEvidenceTagMenu(menu);
          setHint(String(e));
        }
      }

      input.addEventListener("input", () => {
        clearTimeout(debounce);
        debounce = setTimeout(runSearch, 180);
      });

      input.addEventListener("keydown", (e) => {
        const items = menu.querySelectorAll(".evidence-tag-menu-item");
        if (e.key === "ArrowDown" && items.length) {
          e.preventDefault();
          activeIndex = Math.min(activeIndex + 1, items.length - 1);
          highlightActive();
          return;
        }
        if (e.key === "ArrowUp" && items.length) {
          e.preventDefault();
          activeIndex = Math.max(activeIndex - 1, 0);
          highlightActive();
          return;
        }
        if (e.key === "Escape") {
          closeEvidenceTagMenu(menu);
          setHint("");
          return;
        }
        if (e.key === "Enter") {
          e.preventDefault();
          if (activeIndex >= 0 && lastNodes[activeIndex]) {
            selectEvidenceTagFromNode(kind, lastNodes[activeIndex], input, menu, hint);
            return;
          }
          addPickToEvidence(input, kind);
          closeEvidenceTagMenu(menu);
          return;
        }
        if (e.key === "Backspace" && !input.value) {
          removeLastEvidenceChip(kind);
        }
      });

      input.addEventListener("blur", () => {
        setTimeout(() => closeEvidenceTagMenu(menu), 180);
      });

      input.addEventListener("focus", () => {
        const q = input.value.trim();
        if (q.length >= 2) runSearch();
        else if (q.length === 1) setHint("Type 2+ letters to search catalog");
      });

      if (box) {
        box.addEventListener("click", (e) => {
          if (e.target.closest(".evidence-chip-remove")) return;
          input.focus();
        });
      }
    }

    wireEvidenceTagField({
      kind: "tool",
      input: document.getElementById("skill-tool-pick"),
      menu: document.getElementById("evidence-tag-menu-tool"),
      hint: document.getElementById("evidence-tag-hint-tool"),
      box: document.getElementById("evidence-tag-box-tool"),
    });
    wireEvidenceTagField({
      kind: "competency",
      input: document.getElementById("skill-comp-pick"),
      menu: document.getElementById("evidence-tag-menu-competency"),
      hint: document.getElementById("evidence-tag-hint-competency"),
      box: document.getElementById("evidence-tag-box-competency"),
    });
    wireEvidenceTagField({
      kind: "tag",
      input: document.getElementById("skill-tag-pick"),
      menu: document.getElementById("evidence-tag-menu-tag"),
      hint: document.getElementById("evidence-tag-hint-tag"),
      box: document.getElementById("evidence-tag-box-marker"),
    });
