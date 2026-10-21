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
        previewThemeId = ids.includes("classic") ? "classic" : ids[0];
        previewThemeSelect.value = previewThemeId;
      }
    }

    async function loadThemeInfo() {
      const themeId = selectedPreviewThemeId();
      const res = await fetch(`/api/themes/${encodeURIComponent(themeId)}`);
      if (res.ok) themeInfo = await res.json();
    }

    async function onPreviewThemeChanged() {
      const theme = selectedPreviewThemeId();
      previewThemeId = theme;
      invalidatePreviewClientState();
      statusEl.textContent = `Building preview (${theme})…`;
      statusEl.classList.remove("error");
      await rebuild();
      loadThemeInfo().catch(() => {});
      if (selectedElementId && !catalogJsonMode.checked) {
        const detail = await getElementDetail(selectedElementId);
        if (detail?.latest?.payload) {
          await renderPayloadForm(detail.kind, detail.latest.payload);
        }
      }
    }
