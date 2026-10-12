# Framework reference imports (F.3b)

Catalog nodes store optional crosswalk metadata in `skill_nodes.framework_refs_json` (merged, not replaced).

## Bundled seed

- [`framework_refs_seed.json`](framework_refs_seed.json) — applied on DB schema **v13** and via CLI.

## Custom imports

JSON array of rows:

```json
[
  {
    "skill_id": "tag.nist-sp-800-53",
    "framework_refs": {
      "nist": { "publication": "SP 800-53", "revision": "Rev. 5" }
    }
  }
]
```

```bash
jobdesk skills import-frameworks --file path/to/your_refs.json
```

Posting keyword hints also scan string values inside `framework_refs` when matching job description text.
