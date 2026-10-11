    async function loadThemeCatalog() {
      const res = await fetch("/api/themes");
      if (!res.ok) return;
      const ids = await res.json();
      previewThemeSelect.innerHTML = "";
      for (const id of ids) {
        const opt = document.createElement("option");
        opt.value = id;
        opt.textContent = id;
        previewThemeSelect.appendChild(opt);
      }
      if (ids.includes(previewThemeId)) {
        previewThemeSelect.value = previewThemeId;
      } else if (ids.length) {
        previewThemeId = ids[0];
        previewThemeSelect.value = previewThemeId;
      }
    }

    async function loadThemeInfo() {
      const res = await fetch(`/api/themes/${encodeURIComponent(previewThemeId)}`);
      if (res.ok) themeInfo = await res.json();
    }

    async function onPreviewThemeChanged() {
      previewThemeId = previewThemeSelect.value || "classic";
      lastPreviewContentHash = null;
      await loadThemeInfo();
      if (selectedElementId && !catalogJsonMode.checked) {
        const detail = await getElementDetail(selectedElementId);
        if (detail?.latest?.payload) {
          await renderPayloadForm(detail.kind, detail.latest.payload);
        }
      }
      await rebuild();
    }

