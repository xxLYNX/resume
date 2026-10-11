    function sectionOrder() {
      let keys;
      if (!themeInfo || !themeInfo.sections) {
        keys = ["header", "objective", "education", "experience", "projects"];
      } else {
        keys = Object.keys(themeInfo.sections);
      }
      // PDF skills come from evidence links on pinned entries, not a composable section.
      return keys.filter((k) => k !== "skills");
    }

    function kindForSection(sectionKey) {
      if (themeInfo?.sections?.[sectionKey]?.element_kind) {
        return themeInfo.sections[sectionKey].element_kind;
      }
      return sectionKey === "projects" ? "project" : sectionKey;
    }

    /** Composition item order first, then catalog elements not in the draft. */
    function catalogElementsOrdered(catalog, sectionItems) {
      if (!sectionItems?.length) return catalog;
      const byId = new Map(catalog.map((el) => [el.id, el]));
      const seen = new Set();
      const ordered = [];
      for (const item of sectionItems) {
        const el = byId.get(item.element_id);
        if (el) {
          ordered.push(el);
          seen.add(el.id);
        }
      }
      for (const el of catalog) {
        if (!seen.has(el.id)) ordered.push(el);
      }
      return ordered;
    }

    function compositionFromEditorDom() {
      const sections = [];
      for (const sectionKey of sectionOrder()) {
        const block = compositionEditor.querySelector(`[data-section="${sectionKey}"]`);
        if (!block) continue;
        const sectionOn = block.querySelector(".section-include")?.checked;
        if (!sectionOn) continue;
        const items = [];
        for (const row of block.querySelectorAll(".comp-row")) {
          const include = row.querySelector(".element-include")?.checked;
          if (!include) continue;
          const elementId = row.dataset.elementId;
          const revisionId = row.querySelector(".comp-revision")?.value;
          if (elementId && revisionId) {
            items.push({ element_id: elementId, revision_id: revisionId });
          }
        }
        if (items.length) sections.push({ section_key: sectionKey, items });
      }
      return { sections };
    }

    function setSectionBodyEnabled(block, enabled) {
      block.classList.toggle("section-off", !enabled);
    }

    async function fillRevisionSelect(select, elementId, selectedRevId) {
      select.innerHTML = "";
      const detail = await getElementDetail(elementId);
      if (!detail?.revisions) return;
      for (const rev of detail.revisions) {
        const opt = document.createElement("option");
        opt.value = rev.id;
        opt.textContent = `rev ${rev.rev_number}`;
        select.appendChild(opt);
      }
      if (selectedRevId && [...select.options].some((o) => o.value === selectedRevId)) {
        select.value = selectedRevId;
      } else if (detail.latest) {
        select.value = detail.latest.id;
      }
    }

    async function renderCompositionEditor(composition) {
      compositionEditor.innerHTML = "";
      const compSections = new Map((composition?.sections || []).map((s) => [s.section_key, s.items || []]));
      for (const sectionKey of sectionOrder()) {
        const wrap = document.createElement("div");
        wrap.className = "composition-section";
        wrap.dataset.section = sectionKey;

        const head = document.createElement("div");
        head.className = "section-head";
        const sectionInclude = document.createElement("input");
        sectionInclude.type = "checkbox";
        sectionInclude.className = "section-include";
        const sectionLabel = document.createElement("label");
        sectionLabel.append(sectionInclude, document.createTextNode(" Include section"));
        const h = document.createElement("h4");
        h.textContent = sectionKey.replace(/_/g, " ");
        head.append(sectionLabel, h);
        wrap.appendChild(head);

        const pinned = new Map(
          (compSections.get(sectionKey) || []).map((it) => [it.element_id, it.revision_id])
        );
        const sectionInComp = compSections.has(sectionKey);
        sectionInclude.checked = sectionInComp;
        setSectionBodyEnabled(wrap, sectionInclude.checked);

        const kind = kindForSection(sectionKey);
        const rowsHost = document.createElement("div");
        rowsHost.className = "section-body comp-rows";
        wrap.appendChild(rowsHost);

        const catalog = catalogByKind[kind] || [];
        const sectionItems = compSections.get(sectionKey) || [];
        const orderedCatalog = catalogElementsOrdered(catalog, sectionItems);
        if (!orderedCatalog.length) {
          const empty = document.createElement("p");
          empty.className = "meta";
          empty.textContent = `No ${kind} entries in portfolio.`;
          rowsHost.appendChild(empty);
        }

        for (const el of orderedCatalog) {
          const row = document.createElement("div");
          row.className = "comp-row";
          row.dataset.elementId = el.id;

          const elInclude = document.createElement("input");
          elInclude.type = "checkbox";
          elInclude.className = "element-include";
          elInclude.checked = pinned.has(el.id);
          const elLabel = document.createElement("label");
          elLabel.append(elInclude, document.createTextNode(" Include"));

          const idSpan = document.createElement("span");
          idSpan.className = "element-id";
          idSpan.textContent = el.id;

          const revSel = document.createElement("select");
          revSel.className = "comp-revision";
          await fillRevisionSelect(revSel, el.id, pinned.get(el.id));

          const up = document.createElement("button");
          up.type = "button";
          up.textContent = "↑";
          const down = document.createElement("button");
          down.type = "button";
          down.textContent = "↓";

          row.append(elLabel, idSpan, revSel, up, down);
          rowsHost.appendChild(row);

          elInclude.addEventListener("change", notifyCompositionChanged);
          revSel.addEventListener("change", notifyCompositionChanged);
          up.addEventListener("click", () => {
            const prev = row.previousElementSibling;
            if (prev?.classList.contains("comp-row")) rowsHost.insertBefore(row, prev);
            notifyCompositionChanged();
          });
          down.addEventListener("click", () => {
            const next = row.nextElementSibling;
            if (next?.classList.contains("comp-row")) rowsHost.insertBefore(next, row);
            notifyCompositionChanged();
          });
        }

        sectionInclude.addEventListener("change", () => {
          setSectionBodyEnabled(wrap, sectionInclude.checked);
          if (sectionInclude.checked) {
            const any = rowsHost.querySelector(".element-include:checked");
            if (!any) {
              rowsHost.querySelectorAll(".element-include").forEach((c) => {
                c.checked = true;
              });
            }
          }
          notifyCompositionChanged();
        });

        compositionEditor.appendChild(wrap);
      }
      notifyCompositionChanged();
    }

