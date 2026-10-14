    async function loadPresets() {
      const status = document.getElementById("presets-status");
      const list = document.getElementById("preset-list");
      status.textContent = "Loading…";
      list.innerHTML = "";
      cloneFromPreset.innerHTML = '<option value="">—</option>';
      try {
        const res = await fetch("/api/presets");
        const data = await res.json();
        if (!res.ok) throw new Error(data.error || "Failed");
        status.textContent = `${data.length} preset(s)`;
        for (const p of data) {
          const li = document.createElement("li");
          li.textContent = p.name;
          li.dataset.id = p.id;
          li.addEventListener("click", () => selectPreset(p.id));
          list.appendChild(li);
          const opt = document.createElement("option");
          opt.value = p.id;
          opt.textContent = p.name;
          cloneFromPreset.appendChild(opt);
        }
      } catch (e) {
        status.textContent = String(e);
      }
    }

    async function selectPreset(id) {
      selectedPresetId = id;
      for (const li of document.getElementById("preset-list").querySelectorAll("li")) {
        li.classList.toggle("active", li.dataset.id === id);
      }
      const res = await fetch(`/api/presets/${encodeURIComponent(id)}`);
      const preset = await res.json();
      if (!res.ok) return;
      document.getElementById("preset-detail").classList.remove("hidden");
      document.getElementById("preset-meta").textContent =
        `${preset.name} · theme ${preset.theme_id} · updated ${preset.updated_at.slice(0, 19)}`;
    }

    function defaultPayloadForKind(kind) {
      switch (kind) {
        case "experience":
          return {
            company: "",
            position: "",
            date_range: { start: { year: 2024, month: 1 }, end: { kind: "present" } },
            highlights: [],
          };
        case "education":
          return {
            degree: "",
            school: "",
            date_range: { start: { year: 2020, month: 9 }, end: { kind: "date", date: { year: 2024, month: 5 } } },
            highlights: [],
          };
        case "project":
          return {
            name: "",
            technologies: "",
            date_range: { start: { year: 2024, month: 1 }, end: { kind: "date", date: { year: 2024, month: 6 } } },
            details: [],
          };
        case "header":
          return { name: "", email: "", phone: "" };
        case "objective":
          return { objective: "" };
        case "course":
          return {
            name: "",
            code: "",
            date_range: {
              start: { year: 2024, month: 1 },
              end: { kind: "date", date: { year: 2024, month: 5 } },
            },
          };
        case "certification":
          return {
            name: "",
            issuer: "",
            date_range: {
              start: { year: 2024, month: 1 },
              end: { kind: "date", date: { year: 2026, month: 12 } },
            },
          };
        case "award":
          return {
            title: "",
            issuer: "",
            date_range: {
              start: { year: 2020, month: 1 },
              end: { kind: "date", date: { year: 2020, month: 6 } },
            },
          };
        default:
          return {};
      }
    }

    async function loadCatalog() {
      catalogStatus.classList.remove("error");
      catalogStatus.textContent = "Loading…";
      try {
        await refreshCatalogByKind();
        await refreshEvidenceSummaries();
        const total = renderPortfolioGroups();
        catalogStatus.textContent = `${total} entr${total === 1 ? "y" : "ies"} in collections`;
      } catch (e) {
        catalogStatus.textContent = String(e);
        catalogStatus.classList.add("error");
      }
    }

