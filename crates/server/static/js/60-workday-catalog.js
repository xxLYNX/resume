    function syncWorkdayFillButtonVisibility(kind) {
      copyWorkdayFillBtn.classList.toggle(
        "hidden",
        kind !== "experience" && kind !== "education",
      );
    }

    function formatExperienceLocationForWorkday(payload) {
      const loc = payload?.location;
      if (!loc || typeof loc !== "object") return "";
      const city = (loc.city || "").trim();
      const state = (loc.state || "").trim();
      if (city && state) return `${city}, ${state}`;
      return city || state;
    }

    function workdayRoleDescriptionFromHighlights(highlights) {
      if (!Array.isArray(highlights)) return "";
      return highlights
        .map((line) => String(line).trim())
        .filter(Boolean)
        .join("\n");
    }

    function workdayFillDataFromExperiencePayload(payload) {
      const dr = payload?.date_range;
      const start = dr?.start;
      const end = dr?.end;
      const currentlyWorkHere = end?.kind === "present";
      let endMonth = null;
      let endYear = null;
      if (!currentlyWorkHere && end?.kind === "date" && end.date) {
        endMonth = end.date.month;
        endYear = end.date.year;
      }
      return {
        jobTitle: (payload?.position || "").trim(),
        companyName: (payload?.company || "").trim(),
        location: formatExperienceLocationForWorkday(payload),
        roleDescription: workdayRoleDescriptionFromHighlights(payload?.highlights),
        startMonth: start?.month ?? null,
        startYear: start?.year ?? null,
        endMonth,
        endYear,
        currentlyWorkHere,
      };
    }

    function buildWorkdayExperienceFillScript(fillData) {
      const dataJson = JSON.stringify(fillData);
      return `// jobdesk → Workday experience (My Experience step on live apply page)
(function () {
  const DATA = ${dataJson};

  function isVisible(el) {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.height > 0 && el.offsetParent !== null;
  }

  function rowKeyFromFkit(root) {
    const fkit = root?.getAttribute("data-fkit-id") || "";
    const sep = fkit.indexOf("--");
    return sep > 0 ? fkit.slice(0, sep) : null;
  }

  function rowKeyFromNode(node) {
    let el = node;
    while (el && el !== document.body) {
      const key = rowKeyFromFkit(el);
      if (key && key.startsWith("workExperience-")) return key;
      el = el.parentElement;
    }
    return null;
  }

  function rowIsEmpty(rowKey) {
    const title = document.querySelector(
      '[data-fkit-id="' + rowKey + '--jobTitle"] input',
    );
    const company = document.querySelector(
      '[data-fkit-id="' + rowKey + '--companyName"] input',
    );
    const t = title?.value?.trim() || "";
    const c = company?.value?.trim() || "";
    return !t && !c;
  }

  /** Prefer focused row; else first visible empty slot (Work Experience 2 before 3); else last row. */
  function pickTargetRowKey() {
    const focused = rowKeyFromNode(document.activeElement);
    if (focused) {
      const titleRoot = document.querySelector(
        '[data-fkit-id="' + focused + '--jobTitle"]',
      );
      if (titleRoot && isVisible(titleRoot)) return focused;
    }

    const jobTitleRoots = [
      ...document.querySelectorAll('[data-automation-id="formField-jobTitle"]'),
    ].filter(isVisible);
    const rows = jobTitleRoots
      .map((root) => ({ root, rowKey: rowKeyFromFkit(root) }))
      .filter((r) => r.rowKey);
    if (!rows.length) return null;

    const empty = rows.filter((r) => rowIsEmpty(r.rowKey));
    if (empty.length) return empty[0].rowKey;

    return rows.at(-1).rowKey;
  }

  function rowRoot(rowKey, fieldSuffix) {
    if (!rowKey) return null;
    const root = document.querySelector('[data-fkit-id="' + rowKey + "--" + fieldSuffix + '"]');
    return root && isVisible(root) ? root : null;
  }

  function setNativeValue(el, value) {
    if (!el) return false;
    const proto =
      el instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
    const desc = Object.getOwnPropertyDescriptor(proto, "value");
    if (!desc?.set) return false;
    desc.set.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  }

  function textControl(rowKey, fieldSuffix) {
    const root = rowRoot(rowKey, fieldSuffix);
    if (!root) return null;
    return root.querySelector("textarea, input:not([type=checkbox])");
  }

  function setWorkdayDateField(rowKey, fieldSuffix, month, year) {
    if (!month || !year) return false;
    const root = rowRoot(rowKey, fieldSuffix);
    if (!root) return false;
    const monthInput = root.querySelector('[data-automation-id="dateSectionMonth-input"]');
    const yearInput = root.querySelector('[data-automation-id="dateSectionYear-input"]');
    const mm = String(month).padStart(2, "0");
    return setNativeValue(monthInput, mm) && setNativeValue(yearInput, String(year));
  }

  function setCheckbox(rowKey, fieldSuffix, checked) {
    const root = rowRoot(rowKey, fieldSuffix);
    const cb = root?.querySelector('input[type="checkbox"]');
    if (!cb) return false;
    if (Boolean(checked) !== cb.checked) cb.click();
    return true;
  }

  const rowKey = pickTargetRowKey();
  if (!rowKey) {
    console.log("[jobdesk] Workday fill: no work history row found (wrong step?)");
    return;
  }

  const filled = [];
  if (DATA.jobTitle && setNativeValue(textControl(rowKey, "jobTitle"), DATA.jobTitle)) {
    filled.push("job title");
  }
  if (DATA.companyName && setNativeValue(textControl(rowKey, "companyName"), DATA.companyName)) {
    filled.push("company");
  }
  if (DATA.location && setNativeValue(textControl(rowKey, "location"), DATA.location)) {
    filled.push("location");
  }
  if (
    DATA.roleDescription &&
    setNativeValue(textControl(rowKey, "roleDescription"), DATA.roleDescription)
  ) {
    filled.push("role description");
  }
  if (setCheckbox(rowKey, "currentlyWorkHere", DATA.currentlyWorkHere)) {
    filled.push(DATA.currentlyWorkHere ? "currently work here" : "currently work here (off)");
  }
  if (setWorkdayDateField(rowKey, "startDate", DATA.startMonth, DATA.startYear)) {
    filled.push("start date");
  }
  if (
    !DATA.currentlyWorkHere &&
    setWorkdayDateField(rowKey, "endDate", DATA.endMonth, DATA.endYear)
  ) {
    filled.push("end date");
  }
  console.log(
    "[jobdesk] Workday fill (" + rowKey + "):",
    filled.length ? filled.join(", ") : "no fields updated",
  );
})();`;
    }

    function workdayFillDataFromEducationPayload(payload) {
      const dr = payload?.date_range;
      const start = dr?.start;
      const end = dr?.end;
      let lastYear = null;
      if (end?.kind === "date" && end.date) {
        lastYear = end.date.year;
      } else if (end?.kind === "present") {
        lastYear = new Date().getFullYear();
      }
      return {
        school: (payload?.school || "").trim(),
        degree: (payload?.degree || "").trim(),
        fieldOfStudy: (payload?.field_of_study || "").trim(),
        firstYear: start?.year ?? null,
        lastYear,
      };
    }

    function buildWorkdayEducationFillScript(fillData) {
      const dataJson = JSON.stringify(fillData);
      return `// jobdesk → Workday education (Education step on live apply page)
(async function () {
  const DATA = ${dataJson};
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  function isVisible(el) {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    const st = window.getComputedStyle(el);
    return st.display !== "none" && st.visibility !== "hidden";
  }

  function rowKeyFromFkit(root) {
    const fkit = root?.getAttribute("data-fkit-id") || "";
    const sep = fkit.indexOf("--");
    return sep > 0 ? fkit.slice(0, sep) : null;
  }

  function rowKeyFromNode(node) {
    let el = node;
    while (el && el !== document.body) {
      const key = rowKeyFromFkit(el);
      if (key && key.startsWith("education-")) return key;
      el = el.parentElement;
    }
    return null;
  }

  function fieldRoot(rowKey, fieldSuffix) {
    if (!rowKey) return null;
    return document.querySelector('[data-fkit-id="' + rowKey + "--" + fieldSuffix + '"]');
  }

  function promptItemsSelected(rowKey, fieldSuffix) {
    const root = fieldRoot(rowKey, fieldSuffix);
    const instr = root?.querySelector('[data-automation-id="promptAriaInstruction"]');
    const txt = instr?.textContent || "";
    if (/0 items selected/i.test(txt)) return 0;
    const m = txt.match(/(\\d+)\\s+items?\\s+selected/i);
    return m ? Number(m[1]) : txt.trim() ? 1 : 0;
  }

  function educationRowIsEmpty(rowKey) {
    const schoolEmpty = promptItemsSelected(rowKey, "school") === 0;
    const btn = fieldRoot(rowKey, "degree")?.querySelector("button");
    const deg = btn?.textContent?.trim() || "";
    return schoolEmpty && (!deg || /^select one$/i.test(deg));
  }

  function listEducationRowKeys() {
    return [
      ...document.querySelectorAll('[data-fkit-id^="education-"][data-fkit-id$="--school"]'),
    ]
      .map((el) => rowKeyFromFkit(el))
      .filter(Boolean)
      .filter((key, i, arr) => arr.indexOf(key) === i)
      .filter((key) => isVisible(fieldRoot(key, "school")));
  }

  function pickTargetRowKey() {
    const focused = rowKeyFromNode(document.activeElement);
    if (focused && fieldRoot(focused, "school")) return focused;

    let rows = listEducationRowKeys();
    if (!rows.length) {
      rows = [...document.querySelectorAll('[data-fkit-id^="education-"][data-fkit-id$="--school"]')]
        .map((el) => rowKeyFromFkit(el))
        .filter(Boolean)
        .filter((key, i, arr) => arr.indexOf(key) === i);
    }
    if (!rows.length) return null;
    const empty = rows.filter((k) => educationRowIsEmpty(k));
    if (empty.length) return empty[0];
    return rows[rows.length - 1];
  }

  function setNativeValue(el, value) {
    if (!el) return false;
    const proto =
      el instanceof HTMLTextAreaElement
        ? HTMLTextAreaElement.prototype
        : HTMLInputElement.prototype;
    const desc = Object.getOwnPropertyDescriptor(proto, "value");
    if (!desc?.set) return false;
    desc.set.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  }

  function pressKey(el, key, keyCode) {
    if (!el) return;
    const opts = { key, code: key, keyCode, which: keyCode, bubbles: true, cancelable: true };
    el.dispatchEvent(new KeyboardEvent("keydown", opts));
    el.dispatchEvent(new KeyboardEvent("keypress", opts));
    el.dispatchEvent(new KeyboardEvent("keyup", opts));
  }

  async function typeLikeUser(input, text) {
    input.focus();
    input.click();
    setNativeValue(input, "");
    for (const ch of text) {
      setNativeValue(input, input.value + ch);
      input.dispatchEvent(
        new InputEvent("input", { bubbles: true, inputType: "insertText", data: ch }),
      );
      pressKey(input, ch, ch.charCodeAt(0));
      await sleep(20);
    }
  }

  function setWorkdayYearField(rowKey, fieldSuffix, year) {
    if (!year) return false;
    const root = fieldRoot(rowKey, fieldSuffix);
    const yearInput = root?.querySelector('[data-automation-id="dateSectionYear-input"]');
    if (!yearInput) return false;
    yearInput.focus();
    yearInput.click();
    const ok = setNativeValue(yearInput, String(year));
    yearInput.dispatchEvent(
      new InputEvent("input", { bubbles: true, inputType: "insertText", data: String(year) }),
    );
    pressKey(yearInput, "Tab", 9);
    yearInput.blur();
    return ok;
  }

  function listPromptOptions() {
    return [
      ...document.querySelectorAll(
        '[data-automation-id="promptOption"], [data-automation-id="menuItem"], [data-automation-id="activeListItem"], [role="option"]',
      ),
    ].filter(isVisible);
  }

  async function pickPromptOption(query) {
    const q = query.toLowerCase();
    const options = listPromptOptions();
    let opt = options.find((o) => o.textContent.trim().toLowerCase() === q);
    if (!opt) opt = options.find((o) => o.textContent.trim().toLowerCase().includes(q));
    if (!opt && q.length > 4) {
      opt = options.find((o) => q.includes(o.textContent.trim().toLowerCase()));
    }
    if (opt) {
      opt.click();
      await sleep(350);
      return true;
    }
    return false;
  }

  async function fillPromptSearch(rowKey, fieldSuffix, query) {
    const root = fieldRoot(rowKey, fieldSuffix);
    if (!root || !query) return false;
    const input =
      root.querySelector('input[data-uxi-widget-type="selectinput"]') ||
      root.querySelector('input[placeholder="Search"]') ||
      root.querySelector("input");
    if (!input) return false;

    root.querySelector('[data-automation-id="multiselectInputContainer"]')?.click();
    root.querySelector('[data-automation-id="promptIcon"]')?.click();
    await sleep(150);
    await typeLikeUser(input, query);
    pressKey(input, "Enter", 13);
    await sleep(1200);
    if (await pickPromptOption(query)) return true;
    pressKey(input, "ArrowDown", 40);
    await sleep(120);
    pressKey(input, "Enter", 13);
    await sleep(400);
    return promptItemsSelected(rowKey, fieldSuffix) > 0;
  }

  async function fillSchool(rowKey, schoolName) {
    if (!schoolName) return false;
    if (await fillPromptSearch(rowKey, "school", schoolName)) return true;
    if (schoolName.toLowerCase() !== "other") {
      return fillPromptSearch(rowKey, "school", "Other");
    }
    return false;
  }

  function degreeSearchTerms(label) {
    const l = label.toLowerCase();
    const terms = [label];
    if (/b\\.s|bachelor/.test(l)) terms.push("Bachelor");
    if (/m\\.s|master/.test(l)) terms.push("Master");
    if (/ph\\.?d|doctorate|doctor/.test(l)) terms.push("Doctor", "PhD");
    if (/associate/.test(l)) terms.push("Associate");
    if (/high school|secondary|ged/.test(l)) terms.push("High School");
    return [...new Set(terms)];
  }

  async function pickDegree(rowKey, degreeLabel) {
    const root = fieldRoot(rowKey, "degree");
    if (!root || !degreeLabel) return false;
    const btn =
      root.querySelector('button[aria-haspopup="listbox"]') ||
      root.querySelector("button[id$='--degree']");
    if (!btn) return false;

    for (const term of degreeSearchTerms(degreeLabel)) {
      btn.focus();
      btn.click();
      await sleep(500);
      const filter = root.querySelector('input[type="text"]');
      if (filter) {
        setNativeValue(filter, term);
        filter.dispatchEvent(new InputEvent("input", { bubbles: true, data: term, inputType: "insertText" }));
        await sleep(350);
      }
      const options = listPromptOptions().concat(
        [...document.querySelectorAll('[role="listbox"] [role="option"], ul[role="listbox"] li')].filter(
          isVisible,
        ),
      );
      const t = term.toLowerCase();
      const opt = options.find((o) => {
        const ot = o.textContent.trim().toLowerCase();
        return ot && (ot.includes(t) || t.includes(ot));
      });
      if (opt) {
        opt.click();
        await sleep(250);
        return true;
      }
      pressKey(document.activeElement || btn, "Escape", 27);
      await sleep(200);
    }
    return !/^select one$/i.test(btn.textContent.trim());
  }

  try {
    const rowKey = pickTargetRowKey();
    if (!rowKey) {
      console.warn("[jobdesk] Workday education fill: no education row found (Education step?)");
      return;
    }

    const filled = [];
    const failed = [];

    if (setWorkdayYearField(rowKey, "firstYearAttended", DATA.firstYear)) filled.push("from year");
    else if (DATA.firstYear) failed.push("from year");

    if (setWorkdayYearField(rowKey, "lastYearAttended", DATA.lastYear)) filled.push("to year");
    else if (DATA.lastYear) failed.push("to year");

    if (DATA.degree && (await pickDegree(rowKey, DATA.degree))) filled.push("degree");
    else if (DATA.degree) failed.push("degree");

    if (DATA.school && (await fillSchool(rowKey, DATA.school))) filled.push("school");
    else if (DATA.school) failed.push("school");

    if (DATA.fieldOfStudy && (await fillPromptSearch(rowKey, "fieldOfStudy", DATA.fieldOfStudy))) {
      filled.push("field of study");
    } else if (DATA.fieldOfStudy) failed.push("field of study");

    console.log(
      "[jobdesk] Workday education fill (" + rowKey + "):",
      filled.length ? filled.join(", ") : "none",
      failed.length ? "| missed: " + failed.join(", ") : "",
    );
  } catch (err) {
    console.error("[jobdesk] Workday education fill error:", err);
  }
})();`;
    }

    function currentElementPayloadForWorkdayFill() {
      const kind = elementCreateKind || selectedCatalogKind;
      if (kind !== "experience" && kind !== "education") return null;
      if (catalogJsonMode.checked) {
        return { kind, payload: JSON.parse(payloadEditor.value || "{}") };
      }
      return { kind, payload: payloadFromForm() };
    }

    async function copyWorkdayFillScript() {
      catalogStatus.classList.remove("error");
      let data;
      try {
        data = currentElementPayloadForWorkdayFill();
      } catch (e) {
        catalogStatus.textContent = "Invalid JSON: " + e.message;
        catalogStatus.classList.add("error");
        return;
      }
      if (!data) {
        catalogStatus.textContent = "Open an experience or education entry first.";
        catalogStatus.classList.add("error");
        return;
      }
      let script;
      let hint;
      if (data.kind === "experience") {
        script = buildWorkdayExperienceFillScript(
          workdayFillDataFromExperiencePayload(data.payload),
        );
        hint = "My Experience step";
      } else {
        script = buildWorkdayEducationFillScript(
          workdayFillDataFromEducationPayload(data.payload),
        );
        hint = "Education step";
      }
      try {
        await navigator.clipboard.writeText(script);
        catalogStatus.textContent =
          "Workday fill script copied — paste on live apply page (" +
          hint +
          "; first empty row or focused row).";
      } catch (e) {
        catalogStatus.textContent = "Clipboard failed: " + e.message;
        catalogStatus.classList.add("error");
      }
    }

