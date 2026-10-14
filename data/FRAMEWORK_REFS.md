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

## NICE / KSA crosswalk (future)

**NIST NICE** work-role KSAs are a natural complement to **O\*NET** (see **Maybe later — NICE KSA enrichment** in [`docs/MILESTONE_F_ROLLOUT.md`](../docs/MILESTONE_F_ROLLOUT.md)). Not shipped in F v1; low-friction path is curated `framework_refs.nice` on existing nodes or a merge import, not a second full catalog replace.
