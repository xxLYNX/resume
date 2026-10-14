# O*NET attribution (CC BY 4.0)

Jobdesk imports occupational skill data from the **O*NET Database** published by the U.S. Department of Labor, Employment and Training Administration (USDOL/ETA), under the [Creative Commons Attribution 4.0 International License](https://creativecommons.org/licenses/by/4.0/).

O*NET® is a trademark of USDOL/ETA. Use the name as an adjective (e.g. “O*NET data”), not as a noun or verb.

## Intended use

- **Download** tab-delimited files from [O*NET Database](https://www.onetcenter.org/database.html) (or run [`scripts/fetch-onet.sh`](../scripts/fetch-onet.sh)).
- **Import** into your local SQLite catalog via `jobdesk skills import-catalog` — we map Technology/Software Skills examples to **tools** and Skills.txt element names to **competencies**; we do not redistribute O*NET files inside this repo.
- **Optional:** [O*NET Web Services / Database Services API](https://www.onetcenter.org/database.html) for always-current data (registration may be required); jobdesk currently uses offline file import.

## Attribution in derivative products

If you ship a product that displays O*NET-derived labels, include attribution appropriate to your edits:

**Verbatim labels (typical for imported catalog nodes):**

> This product includes information from the O*NET Database by the U.S. Department of Labor, Employment and Training Administration (USDOL/ETA). Used under the CC BY 4.0 license. O*NET® is a trademark of USDOL/ETA.

**If you modify O*NET-derived content:**

> … [Your project] has modified all or some of this information. USDOL/ETA has not approved, endorsed, or tested these modifications.

Link to the [license](https://www.onetcenter.org/license.html) where you display credits.
