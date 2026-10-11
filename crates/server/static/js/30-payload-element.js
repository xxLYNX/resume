    function getNested(obj, path) {
      const parts = path.split(".");
      let cur = obj;
      for (const p of parts) {
        if (cur == null) return undefined;
        cur = cur[p];
      }
      return cur;
    }

    function setNested(obj, path, value) {
      const parts = path.split(".");
      let cur = obj;
      for (let i = 0; i < parts.length - 1; i++) {
        const p = parts[i];
        if (typeof cur[p] !== "object" || cur[p] === null) cur[p] = {};
        cur = cur[p];
      }
      cur[parts[parts.length - 1]] = value;
    }

    const MONTH_ABBR = [
      "Jan.", "Feb.", "Mar.", "Apr.", "May", "Jun.",
      "Jul.", "Aug.", "Sep.", "Oct.", "Nov.", "Dec.",
    ];

    function defaultDateParts(field) {
      const p = field.date_parts;
      return p && p.length ? p : ["month", "year"];
    }

    function readCalendarDateFromForm(fieldKey, part, dateParts) {
      const q = (comp) =>
        payloadForm.querySelector(
          `[data-field-key="${fieldKey}"][data-date-range-part="${part}"][data-date-component="${comp}"]`,
        );
      const yearRaw = q("year")?.value;
      const year = yearRaw ? parseInt(yearRaw, 10) : NaN;
      if (!year || year < 1900 || year > 2200) return null;
      const out = { year };
      if (dateParts.includes("month")) {
        const month = parseInt(q("month")?.value || "", 10);
        if (!month || month < 1 || month > 12) return null;
        out.month = month;
      }
      if (dateParts.includes("day")) {
        const day = parseInt(q("day")?.value || "", 10);
        if (!day || day < 1 || day > 31) return null;
        out.day = day;
      }
      return out;
    }

    function calendarDateSortKey(d, dateParts) {
      if (!d || !d.year) return 0;
      const month = dateParts.includes("month") ? (d.month || 1) : 1;
      const day = dateParts.includes("day") ? (d.day || 1) : 1;
      return d.year * 10000 + month * 100 + day;
    }

    function monthDisplayToken(month, monthDisplay) {
      if (monthDisplay === "numeric") return String(month);
      return MONTH_ABBR[month - 1] || String(month);
    }

    function formatCalendarDatePreview(d, dateParts, monthDisplay) {
      if (!d || d.year == null) return "";
      const bits = [];
      if (dateParts.includes("day") && d.day != null) bits.push(String(d.day));
      if (dateParts.includes("month") && d.month != null) {
        bits.push(monthDisplayToken(d.month, monthDisplay || "abbrev"));
      }
      if (dateParts.includes("year")) bits.push(String(d.year));
      return bits.join(" ");
    }

    function formatDateRangePreview(range, dateParts, monthDisplay) {
      if (!range?.start) return "";
      const start = formatCalendarDatePreview(range.start, dateParts, monthDisplay);
      if (!start) return "";
      let end;
      if (range.end?.kind === "present") {
        end = "Present";
      } else if (range.end?.kind === "date" && range.end.date) {
        end = formatCalendarDatePreview(range.end.date, dateParts, monthDisplay);
      } else {
        return start;
      }
      return `${start} -- ${end}`;
    }

    function readDateRangeFromForm(fieldKey, dateParts) {
      const presentEl = payloadForm.querySelector(
        `[data-date-range-part="present"][data-field-key="${fieldKey}"]`,
      );
      const start = readCalendarDateFromForm(fieldKey, "start", dateParts);
      if (!start) return null;
      if (presentEl?.checked) {
        return { start, end: { kind: "present" } };
      }
      const endDate = readCalendarDateFromForm(fieldKey, "end", dateParts);
      if (!endDate) return null;
      return { start, end: { kind: "date", date: endDate } };
    }

    function readStringListFromForm(fieldKey) {
      const rows = payloadForm.querySelectorAll(
        `.string-list-row[data-field-key="${fieldKey}"]`,
      );
      const lines = [];
      for (const row of rows) {
        const input = row.querySelector('input[type="text"]');
        const v = input?.value.trim();
        if (v) lines.push(v);
      }
      return lines;
    }

    function readWorkLocationFromForm(fieldKey) {
      const city = payloadForm
        .querySelector(`[data-work-location-part="city"][data-field-key="${fieldKey}"]`)
        ?.value.trim();
      const state = payloadForm
        .querySelector(`[data-work-location-part="state"][data-field-key="${fieldKey}"]`)
        ?.value.trim();
      if (!city && !state) return null;
      const out = {};
      if (city) out.city = city;
      if (state) out.state = state;
      return out;
    }

    function appendWorkLocationField(field, payload) {
      const fs = document.createElement("fieldset");
      fs.className = "work-location-field";
      const legend = document.createElement("legend");
      legend.textContent = field.label + (field.optional ? " (optional)" : "");
      fs.appendChild(legend);
      const loc = getNested(payload, field.key) || {};
      const row = document.createElement("div");
      row.className = "date-part-row";
      for (const [part, label, placeholder] of [
        ["city", "City / locality", "e.g. Leesburg"],
        ["state", "State / region", "e.g. VA"],
      ]) {
        const wrap = document.createElement("label");
        wrap.textContent = label;
        const input = document.createElement("input");
        input.type = "text";
        input.placeholder = placeholder;
        input.dataset.fieldKey = field.key;
        input.dataset.workLocationPart = part;
        input.value = loc[part] || "";
        wrap.appendChild(input);
        row.appendChild(wrap);
      }
      fs.appendChild(row);
      payloadForm.appendChild(fs);
    }

    function appendStringListRow(host, fieldKey, value = "") {
      const row = document.createElement("div");
      row.className = "string-list-row";
      row.dataset.fieldKey = fieldKey;
      const input = document.createElement("input");
      input.type = "text";
      input.value = value;
      input.placeholder = "One bullet — plain text, no leading dash required";
      const removeBtn = document.createElement("button");
      removeBtn.type = "button";
      removeBtn.className = "string-list-remove";
      removeBtn.textContent = "Remove";
      removeBtn.addEventListener("click", () => row.remove());
      row.append(input, removeBtn);
      host.appendChild(row);
      return input;
    }

    function appendStringListField(field, payload) {
      const fs = document.createElement("fieldset");
      fs.className = "string-list-field";
      const legend = document.createElement("legend");
      legend.textContent = field.label + (field.optional ? " (optional)" : "");
      fs.appendChild(legend);
      const host = document.createElement("div");
      host.className = "string-list-rows";
      host.dataset.fieldKey = field.key;
      const val = getNested(payload, field.key);
      const items = Array.isArray(val) ? val.filter((x) => typeof x === "string") : [];
      if (items.length === 0) {
        appendStringListRow(host, field.key, "");
      } else {
        for (const item of items) {
          appendStringListRow(host, field.key, item);
        }
      }
      fs.appendChild(host);
      const addBtn = document.createElement("button");
      addBtn.type = "button";
      addBtn.className = "string-list-add";
      addBtn.textContent = "Add bullet";
      addBtn.addEventListener("click", () => {
        appendStringListRow(host, field.key, "").focus();
      });
      fs.appendChild(addBtn);
      payloadForm.appendChild(fs);
    }

    function appendCalendarDateControls(container, fieldKey, part, date, dateParts) {
      const row = document.createElement("div");
      row.className = "date-part-row";
      if (dateParts.includes("month")) {
        const monthLabel = document.createElement("label");
        monthLabel.textContent = "Month";
        const monthSel = document.createElement("select");
        monthSel.dataset.fieldKey = fieldKey;
        monthSel.dataset.dateRangePart = part;
        monthSel.dataset.dateComponent = "month";
        const empty = document.createElement("option");
        empty.value = "";
        empty.textContent = "—";
        monthSel.appendChild(empty);
        for (let m = 1; m <= 12; m++) {
          const opt = document.createElement("option");
          opt.value = String(m);
          opt.textContent = MONTH_ABBR[m - 1].replace(".", "");
          monthSel.appendChild(opt);
        }
        if (date?.month) monthSel.value = String(date.month);
        monthLabel.appendChild(monthSel);
        row.appendChild(monthLabel);
      }
      if (dateParts.includes("year")) {
        const yearLabel = document.createElement("label");
        yearLabel.textContent = "Year";
        const yearInput = document.createElement("input");
        yearInput.type = "number";
        yearInput.min = "1950";
        yearInput.max = "2100";
        yearInput.step = "1";
        yearInput.placeholder = "YYYY";
        yearInput.dataset.fieldKey = fieldKey;
        yearInput.dataset.dateRangePart = part;
        yearInput.dataset.dateComponent = "year";
        if (date?.year) yearInput.value = String(date.year);
        yearLabel.appendChild(yearInput);
        row.appendChild(yearLabel);
      }
      if (dateParts.includes("day")) {
        const dayLabel = document.createElement("label");
        dayLabel.textContent = "Day";
        const dayInput = document.createElement("input");
        dayInput.type = "number";
        dayInput.min = "1";
        dayInput.max = "31";
        dayInput.dataset.fieldKey = fieldKey;
        dayInput.dataset.dateRangePart = part;
        dayInput.dataset.dateComponent = "day";
        if (date?.day) dayInput.value = String(date.day);
        dayLabel.appendChild(dayInput);
        row.appendChild(dayLabel);
      }
      container.appendChild(row);
    }

    function bindDateRangePreview(fieldKey, dateParts, monthDisplay, previewEl, endBlock) {
      const refresh = () => {
        const presentEl = payloadForm.querySelector(
          `[data-date-range-part="present"][data-field-key="${fieldKey}"]`,
        );
        const isPresent = presentEl?.checked;
        endBlock.hidden = isPresent;
        const range = readDateRangeFromForm(fieldKey, dateParts);
        if (!range) {
          previewEl.textContent = "Enter start month and year.";
          previewEl.classList.remove("error");
          return;
        }
        if (
          !isPresent &&
          range.end?.kind === "date" &&
          calendarDateSortKey(range.end.date, dateParts) <
            calendarDateSortKey(range.start, dateParts)
        ) {
          previewEl.textContent = "End date is before start date.";
          previewEl.classList.add("error");
          return;
        }
        previewEl.textContent =
          "Preview: " + formatDateRangePreview(range, dateParts, monthDisplay);
        previewEl.classList.remove("error");
      };
      payloadForm.querySelectorAll(`[data-field-key="${fieldKey}"]`).forEach((el) => {
        el.addEventListener("change", refresh);
        el.addEventListener("input", refresh);
      });
      refresh();
    }

    function payloadFromForm() {
      const payload = {};
      for (const field of currentFieldSchema) {
        if (field.field_type === "date_range") {
          const dateParts = defaultDateParts(field);
          const range = readDateRangeFromForm(field.key, dateParts);
          if (range) setNested(payload, field.key, range);
          continue;
        }
        if (field.field_type === "work_location") {
          const loc = readWorkLocationFromForm(field.key);
          if (loc) setNested(payload, field.key, loc);
          continue;
        }
        if (field.field_type === "string_list") {
          const lines = readStringListFromForm(field.key);
          if (lines.length) setNested(payload, field.key, lines);
          continue;
        }
        const el = payloadForm.querySelector(`[data-field-key="${field.key}"]`);
        if (!el) continue;
        if (field.field_type === "text" || field.field_type === "string") {
          const v = el.value.trim();
          if (v) setNested(payload, field.key, v);
        }
      }
      return payload;
    }

    async function renderPayloadForm(kind, payload) {
      currentFieldSchema = [];
      payloadForm.innerHTML = "";
      const res = await fetch(
        `/api/catalog/field-schema?kind=${encodeURIComponent(kind)}&theme=${encodeURIComponent(previewThemeId)}`,
      );
      if (!res.ok) return;
      currentFieldSchema = await res.json();
      for (const field of currentFieldSchema) {
        if (field.field_type === "work_location") {
          appendWorkLocationField(field, payload);
          continue;
        }
        if (field.field_type === "date_range") {
          const dateParts = defaultDateParts(field);
          const range = getNested(payload, field.key);
          const fs = document.createElement("fieldset");
          fs.className = "date-range-field";
          const legend = document.createElement("legend");
          legend.textContent = field.label + (field.optional ? " (optional)" : "");
          fs.appendChild(legend);

          const grid = document.createElement("div");
          grid.className = "date-range-grid";

          const startBlock = document.createElement("div");
          startBlock.className = "date-range-block";
          const startTitle = document.createElement("div");
          startTitle.className = "block-label";
          startTitle.textContent = "Start";
          startBlock.appendChild(startTitle);
          appendCalendarDateControls(startBlock, field.key, "start", range?.start, dateParts);

          const endBlock = document.createElement("div");
          endBlock.className = "date-range-block";
          const endTitle = document.createElement("div");
          endTitle.className = "block-label";
          endTitle.textContent = "End";
          endBlock.appendChild(endTitle);
          const endDate =
            range?.end?.kind === "date" ? range.end.date : null;
          appendCalendarDateControls(endBlock, field.key, "end", endDate, dateParts);

          grid.append(startBlock, endBlock);
          fs.appendChild(grid);

          const presentLabel = document.createElement("label");
          presentLabel.className = "date-range-present";
          const presentCb = document.createElement("input");
          presentCb.type = "checkbox";
          presentCb.dataset.fieldKey = field.key;
          presentCb.dataset.dateRangePart = "present";
          presentCb.checked = !range || range.end?.kind === "present";
          presentLabel.append(presentCb, document.createTextNode(" Ongoing (Present)"));
          fs.appendChild(presentLabel);

          const preview = document.createElement("div");
          preview.className = "date-range-preview";
          fs.appendChild(preview);

          payloadForm.appendChild(fs);
          bindDateRangePreview(
            field.key,
            dateParts,
            field.month_display || "abbrev",
            preview,
            endBlock,
          );
          continue;
        }
        if (field.field_type === "string_list") {
          appendStringListField(field, payload);
          continue;
        }
        const label = document.createElement("label");
        label.textContent = field.label + (field.optional ? " (optional)" : "");
        let input;
        if (field.field_type === "text") {
          input = document.createElement("textarea");
          input.rows = 3;
          input.value = getNested(payload, field.key) || "";
        } else {
          input = document.createElement("input");
          input.type = "text";
          input.value = getNested(payload, field.key) || "";
        }
        input.dataset.fieldKey = field.key;
        label.appendChild(input);
        payloadForm.appendChild(label);
      }
      attachPayloadIdSuggestListeners();
    }

    function slugifyLabel(s) {
      return String(s)
        .toLowerCase()
        .trim()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-+|-+$/g, "")
        .slice(0, 48);
    }

    function suggestElementIdFromPayload(kind, payload) {
      let slug = "";
      if (kind === "experience") {
        const company = payload.company || payload.position;
        const dr = payload.date_range;
        const dateHint =
          dr?.start && `${dr.start.year}-${dr.start.month}`;
        if (company && dateHint) {
          slug = `${slugifyLabel(company)}-${slugifyLabel(dateHint)}`;
        } else if (company) {
          slug = slugifyLabel(company);
        }
      } else if (kind === "project" && payload.name) {
        slug = slugifyLabel(payload.name);
      } else if (kind === "education") {
        const bit = payload.school || payload.degree;
        if (bit) slug = slugifyLabel(bit);
      } else if (kind === "header" && payload.name) {
        slug = slugifyLabel(payload.name);
      }
      if (slug) return `${kind}.${slug}`;
      return `${kind}.new-${Date.now().toString(36).slice(-6)}`;
    }

    function maybeSuggestElementId() {
      if (!elementCreateKind || userEditedElementId) return;
      try {
        const payload = catalogJsonMode.checked
          ? JSON.parse(payloadEditor.value || "{}")
          : payloadFromForm();
        catalogElementIdInput.value = suggestElementIdFromPayload(elementCreateKind, payload);
      } catch {
        /* ignore partial form */
      }
    }

    function onPayloadInputForIdSuggest() {
      if (!userEditedElementId) maybeSuggestElementId();
    }

    function attachPayloadIdSuggestListeners() {
      if (!elementCreateKind) return;
      for (const el of payloadForm.querySelectorAll(
        "[data-field-key], .string-list-row input",
      )) {
        el.removeEventListener("input", onPayloadInputForIdSuggest);
        el.addEventListener("input", onPayloadInputForIdSuggest);
      }
    }

    function isElementCreateMode() {
      return elementCreateKind != null;
    }

    function closeElementDetail() {
      elementCreateKind = null;
      selectedCatalogKind = null;
      selectedElementKind = null;
      copyWorkdayFillBtn.classList.add("hidden");
      syncSkillLinksPanelVisibility(null);
      selectedElementId = null;
      userEditedElementId = false;
      catalogListBody.classList.remove("hidden");
      catalogElementDetail.classList.add("hidden");
      catalogCreateIdRow.classList.add("hidden");
      catalogCreateParentRow.classList.add("hidden");
      createElementBtn.classList.add("hidden");
      deleteElementBtn.classList.add("hidden");
      saveBtn.classList.remove("hidden");
      catalogRevisionLabel.classList.remove("hidden");
      catalogCreateHint.classList.add("hidden");
      includeInAppWrap.classList.add("hidden");
      payloadEditor.disabled = true;
      payloadEditor.value = "";
      payloadForm.innerHTML = "";
      clearPortfolioActive();
    }

    async function openNewElementForm(kind) {
      elementCreateKind = kind;
      selectedElementId = null;
      userEditedElementId = false;
      newEntryKindSelect.value = kind;
      catalogListBody.classList.add("hidden");
      catalogElementDetail.classList.remove("hidden");
      catalogCreateIdRow.classList.remove("hidden");
      createElementBtn.classList.remove("hidden");
      deleteElementBtn.classList.add("hidden");
      saveBtn.classList.add("hidden");
      catalogRevisionLabel.classList.add("hidden");
      catalogCreateHint.classList.remove("hidden");
      catalogElementMeta.textContent = `New ${kind} entry`;
      selectedCatalogKind = kind;
      selectedElementKind = kind;
      syncWorkdayFillButtonVisibility(kind);
      syncSkillLinksPanelVisibility(kind);
      syncCreateParentRow(kind);
      setSkillLinkSelections([]);
      catalogStatus.textContent = "";
      catalogStatus.classList.remove("error");
      revisionSelect.innerHTML = "";
      revisionSelect.disabled = true;
      payloadEditor.disabled = false;
      const payload = defaultPayloadForKind(kind);
      setEditorPayload(payload);
      await renderPayloadForm(kind, payload);
      syncCatalogEditorVisibility();
      catalogElementIdInput.value = suggestElementIdFromPayload(kind, payload);
      clearPortfolioActive();
      syncIncludeInAppWrap();
    }

    function syncIncludeInAppWrap() {
      const show = isElementCreateMode() && selectedAppId && !isCreateMode();
      includeInAppWrap.classList.toggle("hidden", !show);
    }

    function sectionKeyForKind(kind) {
      for (const key of sectionOrder()) {
        if (kindForSection(key) === kind) return key;
      }
      return kind === "project" ? "projects" : kind;
    }

    async function addElementToApplicationDraft(appId, elementId, kind) {
      elementDetailCache.delete(elementId);
      const detail = await getElementDetail(elementId);
      if (!detail?.latest?.id) throw new Error("No revision for new entry");
      const sectionKey = sectionKeyForKind(kind);
      const res = await fetch(`/api/applications/${encodeURIComponent(appId)}`);
      const app = await res.json();
      if (!res.ok) throw new Error(app.error || "Load application failed");
      const comp = JSON.parse(JSON.stringify(app.draft_composition || { sections: [] }));
      comp.sections = comp.sections || [];
      let sec = comp.sections.find((s) => s.section_key === sectionKey);
      if (!sec) {
        sec = { section_key: sectionKey, items: [] };
        comp.sections.push(sec);
      }
      sec.items = sec.items || [];
      if (!sec.items.some((it) => it.element_id === elementId)) {
        sec.items.push({ element_id: elementId, revision_id: detail.latest.id });
      }
      const patch = await fetch(`/api/applications/${encodeURIComponent(appId)}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ draft_composition: comp }),
      });
      const updated = await patch.json();
      if (!patch.ok) throw new Error(updated.error || "Could not add to application");
      if (selectedAppId === appId && activeMainTab === "apps") {
        draftEditor.value = JSON.stringify(updated.draft_composition, null, 2);
        await renderCompositionEditor(updated.draft_composition);
        notifyCompositionChanged();
      }
    }

    async function refreshOpenCompositionRevisionSelects(elementId) {
      const rows = compositionEditor.querySelectorAll(".comp-row");
      if (!rows.length) return;
      for (const row of rows) {
        const elId = row.dataset.elementId;
        if (elementId && elId !== elementId) continue;
        const revSel = row.querySelector(".comp-revision");
        if (!revSel) continue;
        elementDetailCache.delete(elId);
        await fillRevisionSelect(revSel, elId, revSel.value);
      }
      if (applicationPreviewActive()) notifyCompositionChanged();
    }

    catalogElementIdInput.addEventListener("input", () => {
      userEditedElementId = true;
    });

    function syncCatalogEditorVisibility() {
      const json = catalogJsonMode.checked;
      payloadEditor.hidden = !json;
      payloadForm.hidden = json;
    }

    catalogJsonMode.addEventListener("change", syncCatalogEditorVisibility);

