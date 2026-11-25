import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  showcasePreviewEnablesApprove,
  showcasePreviewApproveEnabled,
  showcaseHasUnsavedEdits,
  showcaseRequiresDraftPreviewMode,
} from "./showcase-approval-state.mjs";

const fixturePath = join(
  dirname(fileURLToPath(import.meta.url)),
  "../../../../crates/domain/tests/fixtures/showcase_approval_cases.json",
);
const fixture = JSON.parse(readFileSync(fixturePath, "utf8"));

function resolve(value, strings) {
  if (value === null || value === undefined) {
    return value;
  }
  if (typeof value === "string" && Object.prototype.hasOwnProperty.call(strings, value)) {
    return strings[value];
  }
  return value;
}

function resolveArgs(args, strings) {
  const out = {};
  for (const [key, value] of Object.entries(args)) {
    out[key] = resolve(value, strings);
  }
  return out;
}

const corePresent = { showcasePreviewEnablesApprove };

for (const c of fixture.cases) {
  test(c.name, () => {
    const args = resolveArgs(c.args, fixture.strings);
    let result;
    switch (c.op) {
      case "hasUnsavedEdits":
        result = showcaseHasUnsavedEdits(args);
        break;
      case "previewEnablesApprove":
        result = showcasePreviewEnablesApprove(args);
        break;
      case "previewApproveEnabled": {
        let core;
        switch (c.rules_core) {
          case "present":
            core = corePresent;
            break;
          case "undefined":
            core = undefined;
            break;
          case "empty":
            core = {};
            break;
          default:
            core = null;
            break;
        }
        result = showcasePreviewApproveEnabled(core, args);
        break;
      }
      case "requiresDraftPreviewMode":
        result = showcaseRequiresDraftPreviewMode(
          args.resumeTargetType,
          args.previewModeValue,
        );
        break;
      default:
        throw new Error(`unknown op ${c.op}`);
    }
    assert.equal(result, c.expect);
  });
}