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

    /** Pinned composition order first, then remaining catalog entries by timeline (recent first). */
    function catalogElementsOrdered(catalog, sectionItems) {
      const sortedCatalog = sortByTimeline(catalog || []);
      if (!sectionItems?.length) return sortedCatalog;
      const byId = new Map(sortedCatalog.map((el) => [el.id, el]));
      const seen = new Set();
      const ordered = [];
      for (const item of sectionItems) {
        const el = byId.get(item.element_id);
        if (el) {
          ordered.push(el);
          seen.add(el.id);
        }
      }
      for (const el of sortedCatalog) {
        if (!seen.has(el.id)) ordered.push(el);
      }
      return ordered;
    }

    function projectsResumeLineTagsMax() {
      const max = themeInfo?.sections?.projects?.resume_line_tags_max;
      return typeof max === "number" && max > 0 ? max : null;
    }

    async function fetchElementSkillLinksForComposition(elementId) {
      const res = await fetch(
        `/api/catalog/elements/${encodeURIComponent(elementId)}/skill-links`,
      );
      if (!res.ok) return [];
      const rows = await res.json();
      return rows.filter((r) => r.source === "asserted");
    }

    function readResumeTagIdsFromRow(row) {
      const raw = row.dataset.resumeTagIds;
      if (!raw) return [];
      try {
        const parsed = JSON.parse(raw);
        return Array.isArray(parsed) ? parsed.filter((id) => typeof id === "string") : [];
      } catch (_) {
        return [];
      }
    }

    function writeResumeTagIdsToRow(row, ids) {
      row.dataset.resumeTagIds = JSON.stringify(ids);
    }

    function updateProjectTagBadge(row) {
      const badge = row.querySelector(".comp-title-tag-count");
      if (!badge) return;
      const n = readResumeTagIdsFromRow(row).length;
      badge.textContent = n ? `${n} on PDF line` : "";
      badge.classList.toggle("hidden", !n);
    }

    function updatePdfLineSummary(row) {
      const summary = row.querySelector(".comp-pdf-line-summary");
      if (!summary) return;
      summary.replaceChildren();
      summary.append(document.createTextNode("PDF subheading preview: "));
      const selected = row.querySelectorAll(".comp-resume-tag-chip.selected");
      const labels = [...selected].map((c) => c.textContent.trim()).filter(Boolean);
      if (!labels.length) {
        const em = document.createElement("em");
        em.textContent = "(none)";
        summary.appendChild(em);
      } else {
        const strong = document.createElement("strong");
        strong.textContent = labels.join(", ");
        summary.appendChild(strong);
      }
    }

    function scrollToCompositionSection(sectionKey) {
      const block = compositionEditor.querySelector(`[data-section="${sectionKey}"]`);
      block?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }

    async function focusProjectRowInComposition(elementId) {
      const block = compositionEditor.querySelector('[data-section="projects"]');
      if (block) {
        const secInc = block.querySelector(".section-include");
        if (secInc && !secInc.checked) {
          secInc.checked = true;
          setSectionBodyEnabled(block, true);
        }
      }
      scrollToCompositionSection("projects");
      const esc =
        typeof CSS !== "undefined" && CSS.escape
          ? CSS.escape(elementId)
          : elementId.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
      const row = compositionEditor.querySelector(
        `.comp-row-project[data-element-id="${esc}"]`,
      );
      if (!row) return;
      const include = row.querySelector(".element-include");
      if (include && !include.checked) include.checked = true;
      syncProjectSubheadingEnabled(row);
      await ensureProjectTagPickerLoaded(row, readResumeTagIdsFromRow(row));
      row.classList.add("comp-row-highlight");
      setTimeout(() => row.classList.remove("comp-row-highlight"), 2500);
      row.scrollIntoView({ block: "nearest", behavior: "smooth" });
    }

    async function projectDisplayName(elementId, revisionId) {
      try {
        const detail = await getElementDetail(elementId);
        const rev =
          detail.revisions?.find((r) => r.id === revisionId) ||
          detail.latest ||
          detail.revisions?.[0];
        const name = rev?.payload?.name;
        if (typeof name === "string" && name.trim()) return name.trim();
      } catch (_) {}
      return elementId;
    }

    async function ensureProjectTagPickerLoaded(row, selectedIds) {
      if (row.dataset.tagsLoaded === "1") {
        updateProjectTagBadge(row);
        return;
      }
      await renderProjectResumeTagPicker(row, selectedIds);
      row.dataset.tagsLoaded = "1";
    }

    function syncProjectSubheadingEnabled(row) {
      const included = !!row.querySelector(".element-include")?.checked;
      const sub = row.querySelector(".comp-project-subheading");
      if (sub) sub.classList.toggle("subheading-off", !included);
      row.querySelectorAll(".comp-resume-tag-chip").forEach((btn) => {
        btn.disabled = !included;
      });
    }

    function projectRowIncluded(row) {
      return !!row.querySelector(".element-include")?.checked;
    }

    async function renderProjectResumeTagPicker(row, selectedIds) {
      const host = row.querySelector(".comp-resume-tags");
      if (!host) return;
      const max = projectsResumeLineTagsMax();
      const links = await fetchElementSkillLinksForComposition(row.dataset.elementId);
      host.innerHTML = "";
      if (!links.length) {
        const hint = document.createElement("span");
        hint.className = "meta comp-resume-tags-empty";
        hint.textContent = "No evidence tags on this project (add in Portfolio).";
        host.appendChild(hint);
        writeResumeTagIdsToRow(row, []);
        updatePdfLineSummary(row);
        return;
      }
      const byId = new Map(links.map((l) => [l.skill_id, l]));
      let order = selectedIds.filter((id) => byId.has(id));
      for (const id of selectedIds) {
        if (!order.includes(id) && byId.has(id)) order.push(id);
      }
      writeResumeTagIdsToRow(row, order);

      const label = document.createElement("span");
      label.className = "comp-resume-tags-label";
      label.textContent = max
        ? `PDF subheading (title line: Name | emphasized tags — pick up to ${max}):`
        : "PDF subheading (title line: Name | emphasized tags):";
      host.appendChild(label);

      const chips = document.createElement("div");
      chips.className = "comp-resume-tag-chips";
      host.appendChild(chips);

      for (const link of links) {
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "comp-resume-tag-chip";
        btn.dataset.skillId = link.skill_id;
        btn.dataset.kind = link.kind || "tool";
        btn.textContent = link.label;
        const on = order.includes(link.skill_id);
        btn.classList.toggle("selected", on);
        btn.setAttribute("aria-pressed", on ? "true" : "false");
        btn.title = on
          ? "Shown after project name on PDF — click to remove"
          : "Not on PDF title line — click to highlight";
        if (max && order.length >= max && !on) {
          btn.classList.add("at-max");
        }
        btn.disabled = !projectRowIncluded(row);
        btn.addEventListener("click", () => {
          if (!projectRowIncluded(row)) return;
          let ids = readResumeTagIdsFromRow(row);
          if (ids.includes(link.skill_id)) {
            ids = ids.filter((id) => id !== link.skill_id);
          } else {
            if (max && ids.length >= max) return;
            ids = [...ids, link.skill_id];
          }
          writeResumeTagIdsToRow(row, ids);
          renderProjectResumeTagPicker(row, ids);
          notifyCompositionChanged();
        });
        chips.appendChild(btn);
      }
      updateProjectTagBadge(row);
      updatePdfLineSummary(row);
      syncProjectSubheadingEnabled(row);
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
            const item = { element_id: elementId, revision_id: revisionId };
            if (sectionKey === "projects") {
              const tagIds = readResumeTagIdsFromRow(row);
              if (tagIds.length) item.resume_tag_skill_ids = tagIds;
            }
            items.push(item);
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

        const pinnedItems = new Map(
          (compSections.get(sectionKey) || []).map((it) => [it.element_id, it])
        );
        const sectionInComp = compSections.has(sectionKey);
        sectionInclude.checked = sectionInComp;
        setSectionBodyEnabled(wrap, sectionInclude.checked);

        if (sectionKey === "projects") {
          const projHelp = document.createElement("p");
          projHelp.className = "comp-section-projects-help";
          projHelp.textContent =
            "Each project lists saved Portfolio tags below its name. Toggle which appear on the PDF title subheading (the emphasized segment after Name | …, like your old technologies line). Include the project, pick tags, then preview on Current draft.";
          wrap.appendChild(projHelp);
        }

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
          const isProject = sectionKey === "projects";
          const row = document.createElement("div");
          row.className = isProject ? "comp-row comp-row-project" : "comp-row";
          row.dataset.elementId = el.id;

          const elInclude = document.createElement("input");
          elInclude.type = "checkbox";
          elInclude.className = "element-include";
          elInclude.checked = pinnedItems.has(el.id);
          const elLabel = document.createElement("label");
          elLabel.append(elInclude, document.createTextNode(" Include"));

          const pinnedItem = pinnedItems.get(el.id);
          const revSel = document.createElement("select");
          revSel.className = "comp-revision";
          await fillRevisionSelect(revSel, el.id, pinnedItem?.revision_id);

          const idSpan = document.createElement("span");
          idSpan.className = "element-id";
          idSpan.textContent = el.id;

          const up = document.createElement("button");
          up.type = "button";
          up.textContent = "↑";
          const down = document.createElement("button");
          down.type = "button";
          down.textContent = "↓";

          let nameSpan;
          let head;
          if (isProject) {
            head = document.createElement("div");
            head.className = "comp-row-head";
            nameSpan = document.createElement("span");
            nameSpan.className = "comp-display-name";
            nameSpan.textContent = await projectDisplayName(
              el.id,
              pinnedItem?.revision_id || revSel.value,
            );
            head.append(elLabel, nameSpan, idSpan, revSel, up, down);
            row.appendChild(head);
          } else {
            row.append(elLabel, idSpan, revSel, up, down);
          }

          if (isProject) {
            const initialTags = pinnedItem?.resume_tag_skill_ids || [];
            writeResumeTagIdsToRow(row, initialTags);

            const badge = document.createElement("span");
            badge.className = "comp-title-tag-count meta hidden";
            head.appendChild(badge);
            updateProjectTagBadge(row);

            const subheading = document.createElement("div");
            subheading.className = "comp-project-subheading";
            const pdfSummary = document.createElement("p");
            pdfSummary.className = "comp-pdf-line-summary meta";
            subheading.appendChild(pdfSummary);
            const tagHost = document.createElement("div");
            tagHost.className = "comp-resume-tags";
            subheading.appendChild(tagHost);
            row.appendChild(subheading);
            updatePdfLineSummary(row);

            await ensureProjectTagPickerLoaded(row, initialTags);
            syncProjectSubheadingEnabled(row);
          }

          rowsHost.appendChild(row);

          elInclude.addEventListener("change", () => {
            if (isProject) syncProjectSubheadingEnabled(row);
            notifyCompositionChanged();
          });
          revSel.addEventListener("change", async () => {
            if (isProject && nameSpan) {
              nameSpan.textContent = await projectDisplayName(el.id, revSel.value);
            }
            notifyCompositionChanged();
          });
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

