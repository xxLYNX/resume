/** Opportunities: ingest job postings without creating applications. */

(function () {
  const panelOpp = document.getElementById("panel-opportunities");
  const oppUrl = document.getElementById("opp-url");
  const oppFetchSave = document.getElementById("opp-fetch-save");
  const oppStatus = document.getElementById("opp-status");
  const oppList = document.getElementById("opp-list");
  const oppDetail = document.getElementById("opp-detail");
  const oppCloseDetail = document.getElementById("opp-close-detail");
  const oppMeta = document.getElementById("opp-meta");
  const oppExtractionLine = document.getElementById("opp-extraction-line");
  const oppExtractionDiagnostics = document.getElementById("opp-extraction-diagnostics");
  const oppReviewLine = document.getElementById("opp-review-line");
  const oppPriorReviewBanner = document.getElementById("opp-prior-review-banner");
  const oppFieldTitle = document.getElementById("opp-field-title");
  const oppFieldEmployer = document.getElementById("opp-field-employer");
  const oppFieldLocation = document.getElementById("opp-field-location");
  const oppFieldEmploymentType = document.getElementById("opp-field-employment-type");
  const oppFieldWorkMode = document.getElementById("opp-field-work-mode");
  const oppFieldPayUnit = document.getElementById("opp-field-pay-unit");
  const oppFieldPayMin = document.getElementById("opp-field-pay-min");
  const oppFieldPayMax = document.getElementById("opp-field-pay-max");
  const oppFieldDescription = document.getElementById("opp-field-description");
  const oppSaveEdits = document.getElementById("opp-save-edits");
  const oppMarkReviewed = document.getElementById("opp-mark-reviewed");
  const oppSourcePreview = document.getElementById("opp-source-preview");

  let selectedPostingId = null;
  let lastDetail = null;

  function mergedPayload(detail) {
    const c = detail?.current;
    return c?.merged || {};
  }

  function renderPriorReview(detail) {
    const cur = detail?.current;
    const prior = detail?.prior_reviewed;
    if (oppPriorReviewBanner) {
      if (cur && !cur.reviewed_at && prior?.reviewed_at) {
        oppPriorReviewBanner.classList.remove("hidden");
        oppPriorReviewBanner.dataset.priorInterpretationId = prior.interpretation_id;
        oppPriorReviewBanner.dataset.priorReviewedAt = prior.reviewed_at;
        oppPriorReviewBanner.textContent =
          "Earlier reviewed interpretation at " +
          prior.reviewed_at +
          " (id " +
          prior.interpretation_id.slice(0, 8) +
          "…). Current extraction is unreviewed.";
      } else {
        oppPriorReviewBanner.classList.add("hidden");
        oppPriorReviewBanner.textContent = "";
        delete oppPriorReviewBanner.dataset.priorInterpretationId;
        delete oppPriorReviewBanner.dataset.priorReviewedAt;
      }
    }
  }

  function fillFormFromDetail(detail) {
    if (!detail) return;
    const m = mergedPayload(detail);
    oppFieldTitle.value = m.title || "";
    oppFieldEmployer.value = m.employer || "";
    oppFieldLocation.value = m.location || "";
    oppFieldEmploymentType.value = m.employment_type || "";
    oppFieldWorkMode.value = m.work_mode || "";
    const comp = m.compensation || {};
    oppFieldPayUnit.value = comp.unit || "";
    oppFieldPayMin.value = comp.min != null ? String(comp.min) : "";
    oppFieldPayMax.value = comp.max != null ? String(comp.max) : "";
    const cur = detail?.current;
    const outcome = cur?.extraction_outcome;
    const userDesc = cur?.user_overrides?.description_plain;
    const machineDesc = cur?.machine_suggested?.description_plain;
    if (userDesc) {
      oppFieldDescription.value = userDesc;
    } else if (
      outcome === "source_unavailable" ||
      outcome === "extraction_failed"
    ) {
      oppFieldDescription.value = "";
      oppFieldDescription.placeholder =
        "No reliable description was extracted — edit manually or consult the raw snapshot below.";
    } else {
      oppFieldDescription.value = m.description_plain || "";
      oppFieldDescription.placeholder = "";
    }
    if (oppExtractionLine) {
      const labels = {
        recognized: "Extraction: recognized posting (verify before review).",
        partial: "Extraction: partial — some fields missing or uncertain.",
        source_unavailable: "Extraction: source reports this posting is unavailable.",
        extraction_failed:
          "Extraction: could not derive useful posting text from HTML.",
      };
      oppExtractionLine.textContent =
        labels[outcome] ||
        "Extraction: saved HTML snapshot; machine fields may be empty.";
    }
    if (oppExtractionDiagnostics) {
      oppExtractionDiagnostics.replaceChildren();
      for (const line of cur?.extraction_diagnostics || []) {
        const li = document.createElement("li");
        li.textContent = line;
        oppExtractionDiagnostics.appendChild(li);
      }
    }
    if (cur?.reviewed_at) {
      oppReviewLine.textContent = "Reviewed at " + cur.reviewed_at;
    } else {
      let line =
        "Not reviewed — extraction is a suggestion until you save edits and mark reviewed.";
      const prior = detail?.prior_reviewed;
      if (prior?.reviewed_at) {
        line +=
          " Previous review at " +
          prior.reviewed_at +
          " (interpretation " +
          prior.interpretation_id.slice(0, 8) +
          "…).";
      }
      oppReviewLine.textContent = line;
    }
    renderPriorReview(detail);
  }

  function userOverridesBody() {
    const minV = oppFieldPayMin.value.trim();
    const maxV = oppFieldPayMax.value.trim();
    const comp = {};
    if (oppFieldPayUnit.value) comp.unit = oppFieldPayUnit.value;
    if (minV !== "") comp.min = parseFloat(minV);
    if (maxV !== "") comp.max = parseFloat(maxV);
    if (!oppFieldPayUnit.value && !minV && !maxV) {
      return {
        title: oppFieldTitle.value.trim() || null,
        employer: oppFieldEmployer.value.trim() || null,
        location: oppFieldLocation.value.trim() || null,
        employment_type: oppFieldEmploymentType.value || null,
        work_mode: oppFieldWorkMode.value || null,
        description_plain: oppFieldDescription.value || null,
        compensation: null,
      };
    }
    return {
      title: oppFieldTitle.value.trim() || null,
      employer: oppFieldEmployer.value.trim() || null,
      location: oppFieldLocation.value.trim() || null,
      employment_type: oppFieldEmploymentType.value || null,
      work_mode: oppFieldWorkMode.value || null,
      description_plain: oppFieldDescription.value || null,
      compensation: comp,
    };
  }

  async function loadOpportunitiesList() {
    try {
      const res = await fetch("/api/opportunities");
      const data = await res.json();
      if (!res.ok) {
        oppStatus.textContent = data.error || "List failed";
        oppStatus.classList.add("error");
        return;
      }
      oppStatus.classList.remove("error");
      oppList.innerHTML = "";
      for (const row of data) {
        const li = document.createElement("li");
        li.setAttribute("data-testid", "opp-list-item");
        let label =
          (row.display_title || row.list_label || "Saved posting") +
          (row.display_company ? " · " + row.display_company : "");
        if (row.prior_reviewed_at) {
          label += " · prior review " + row.prior_reviewed_at;
        }
        const urlHint =
          row.normalized_url.length > 72
            ? row.normalized_url.slice(0, 72) + "…"
            : row.normalized_url;
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "linklike";
        btn.textContent = label;
        const meta = document.createElement("span");
        meta.className = "meta";
        meta.textContent = " " + urlHint;
        li.appendChild(btn);
        li.appendChild(meta);
        btn.addEventListener("click", () => openPosting(row.id));
        oppList.appendChild(li);
      }
    } catch (e) {
      oppStatus.textContent = String(e);
      oppStatus.classList.add("error");
    }
  }

  async function loadSnapshotSource(detail) {
    const snap = detail.snapshots?.[detail.snapshots.length - 1];
    if (!snap) {
      oppSourcePreview.textContent = "";
      return;
    }
    const sr = await fetch(
      "/api/opportunities/snapshots/" + encodeURIComponent(snap.id) + "/source",
    );
    const txt = await sr.text();
    if (sr.ok) {
      oppSourcePreview.textContent = txt.slice(0, 120000);
      if (txt.length > 120000) {
        oppSourcePreview.textContent += "\n… truncated for browser …";
      }
    } else {
      oppSourcePreview.textContent = "(could not load source)";
    }
  }

  async function applyPostingDetail(detail) {
    if (!detail?.posting?.id) return;
    selectedPostingId = detail.posting.id;
    oppDetail.classList.remove("hidden");
    lastDetail = detail;
    const p = detail.posting;
    oppMeta.replaceChildren();
    const strong = document.createElement("strong");
    strong.textContent = p.display_title || p.list_label || "Saved posting";
    oppMeta.appendChild(strong);
    if (p.display_company) {
      oppMeta.appendChild(document.createTextNode(" · " + p.display_company));
    }
    const code = document.createElement("code");
    code.textContent = " · " + p.id;
    oppMeta.appendChild(code);
    fillFormFromDetail(detail);
    await loadSnapshotSource(detail);
  }

  async function openPosting(id) {
    oppDetail.classList.remove("hidden");
    try {
      const res = await fetch("/api/opportunities/" + encodeURIComponent(id));
      const detail = await res.json();
      if (!res.ok) {
        oppMeta.textContent = detail.error || "Load failed";
        return;
      }
      await applyPostingDetail(detail);
    } catch (e) {
      oppMeta.textContent = String(e);
    }
  }

  oppFetchSave.addEventListener("click", async () => {
    const url = oppUrl.value.trim();
    if (!url) {
      oppStatus.textContent = "Enter a URL";
      oppStatus.classList.add("error");
      return;
    }
    oppFetchSave.disabled = true;
    oppStatus.textContent = "Fetching…";
    oppStatus.classList.remove("error");
    try {
      const res = await fetch("/api/opportunities/ingest", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url }),
      });
      const data = await res.json();
      if (!res.ok) {
        oppStatus.textContent = data.error || "Ingest failed";
        oppStatus.classList.add("error");
        return;
      }
      const oc = data.posting?.current?.extraction_outcome;
      const hint =
        oc === "source_unavailable"
          ? " (source unavailable — fill fields manually)"
          : oc === "extraction_failed"
            ? " (extraction failed — raw HTML saved)"
            : oc === "partial"
              ? " (partial extraction)"
              : "";
      await loadOpportunitiesList();
      await applyPostingDetail(data.posting);
      oppStatus.textContent = "Saved posting " + data.posting.posting.id + hint;
    } catch (e) {
      oppStatus.textContent = String(e);
      oppStatus.classList.add("error");
    } finally {
      oppFetchSave.disabled = false;
    }
  });

  oppCloseDetail.addEventListener("click", () => {
    oppDetail.classList.add("hidden");
    selectedPostingId = null;
    if (oppPriorReviewBanner) {
      oppPriorReviewBanner.classList.add("hidden");
    }
  });

  oppSaveEdits.addEventListener("click", async () => {
    if (!selectedPostingId) return;
    oppSaveEdits.disabled = true;
    try {
      const res = await fetch(
        "/api/opportunities/" + encodeURIComponent(selectedPostingId) + "/interpretation",
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ user_overrides: userOverridesBody() }),
        },
      );
      const data = await res.json();
      if (!res.ok) {
        oppStatus.textContent = data.error || "Save failed";
        oppStatus.classList.add("error");
        return;
      }
      oppStatus.textContent = "Interpretation updated";
      oppStatus.classList.remove("error");
      lastDetail = data;
      fillFormFromDetail(data);
      await loadOpportunitiesList();
    } catch (e) {
      oppStatus.textContent = String(e);
      oppStatus.classList.add("error");
    } finally {
      oppSaveEdits.disabled = false;
    }
  });

  oppMarkReviewed.addEventListener("click", async () => {
    if (!selectedPostingId) return;
    oppMarkReviewed.disabled = true;
    try {
      const res = await fetch(
        "/api/opportunities/" + encodeURIComponent(selectedPostingId) + "/interpretation",
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ mark_reviewed: true }),
        },
      );
      const data = await res.json();
      if (!res.ok) {
        oppStatus.textContent = data.error || "Save failed";
        oppStatus.classList.add("error");
        return;
      }
      lastDetail = data;
      fillFormFromDetail(data);
      oppStatus.textContent = "Marked reviewed";
      oppStatus.classList.remove("error");
      await loadOpportunitiesList();
    } catch (e) {
      oppStatus.textContent = String(e);
      oppStatus.classList.add("error");
    } finally {
      oppMarkReviewed.disabled = false;
    }
  });

  window.loadOpportunitiesList = loadOpportunitiesList;
})();
