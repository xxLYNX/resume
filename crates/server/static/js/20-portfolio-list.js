    async function refreshCatalogByKind() {
      catalogByKind = {};
      const kinds = [
        "header",
        "objective",
        "education",
        "experience",
        "project",
        "course",
        "certification",
        "award",
      ];
      for (const kind of kinds) {
        const res = await fetch(`/api/catalog/elements?kind=${encodeURIComponent(kind)}`);
        if (res.ok) catalogByKind[kind] = await res.json();
      }
    }

    function sortByTimeline(elements) {
      return [...(elements || [])].sort(
        (a, b) => (b.timeline_sort_key || 0) - (a.timeline_sort_key || 0),
      );
    }

    function clearPortfolioActive() {
      for (const row of portfolioGroups.querySelectorAll(".portfolio-element")) {
        row.classList.remove("active");
      }
    }

    function setPortfolioActive(id) {
      clearPortfolioActive();
      const row = portfolioGroups.querySelector(`.portfolio-element[data-id="${CSS.escape(id)}"]`);
      if (row) row.classList.add("active");
    }

    function renderPortfolioElement(el, { nested = false } = {}) {
      const row = document.createElement("div");
      row.className = "portfolio-element" + (nested ? " nested" : "");
      row.dataset.id = el.id;
      const title = el.display_label || el.id;
      row.innerHTML = `<span class="meta-line">${el.kind} · rev ${el.latest_rev_number}</span>${title}`;
      row.addEventListener("click", () => selectElement(el.id));
      return row;
    }

    function renderPortfolioGroups() {
      portfolioGroups.innerHTML = "";
      let total = 0;
      for (const col of PORTFOLIO_COLLECTIONS) {
        const items = [];
        for (const kind of col.kinds) {
          const list = catalogByKind[kind] || [];
          if (col.id === "education") {
            items.push(...list.filter((el) => !el.parent_element_id));
          } else {
            items.push(...list);
          }
        }
        const sorted = sortByTimeline(items);
        const courses = col.nestedKind ? sortByTimeline(catalogByKind[col.nestedKind] || []) : [];
        if (sorted.length === 0 && courses.length === 0) continue;
        total += sorted.length + courses.length;

        const details = document.createElement("details");
        details.className = "portfolio-collection";
        const summary = document.createElement("summary");
        summary.innerHTML = `${col.label}<span class="portfolio-count">${sorted.length + (col.nestedKind ? ` + ${courses.length} classes` : "")}</span>`;
        details.appendChild(summary);

        for (const el of sorted) {
          details.appendChild(renderPortfolioElement(el));
          if (col.nestedKind) {
            const kids = courses.filter((c) => c.parent_element_id === el.id);
            for (const course of kids) {
              details.appendChild(renderPortfolioElement(course, { nested: true }));
            }
          }
        }
        // Orphan courses (missing parent)
        if (col.nestedKind) {
          const orphans = courses.filter(
            (c) => !sorted.some((p) => p.id === c.parent_element_id),
          );
          for (const course of orphans) {
            details.appendChild(renderPortfolioElement(course, { nested: true }));
          }
        }
        portfolioGroups.appendChild(details);
      }
      return total;
    }

    function fillEducationParentSelect() {
      catalogParentElementSelect.innerHTML = "";
      const programs = sortByTimeline(catalogByKind.education || []);
      for (const el of programs) {
        const opt = document.createElement("option");
        opt.value = el.id;
        opt.textContent = el.display_label ? `${el.display_label} (${el.id})` : el.id;
        catalogParentElementSelect.appendChild(opt);
      }
    }

    function syncCreateParentRow(kind) {
      const show = kind === "course";
      catalogCreateParentRow.classList.toggle("hidden", !show);
      if (show) fillEducationParentSelect();
    }

    async function getElementDetail(id) {
      if (elementDetailCache.has(id)) return elementDetailCache.get(id);
      const res = await fetch(`/api/catalog/elements/${encodeURIComponent(id)}`);
      const detail = await res.json();
      if (res.ok) elementDetailCache.set(id, detail);
      return detail;
    }

