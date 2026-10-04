import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { matchesLog, parseQuery, queryLog } from "./query-runtime-log.mjs";
const event = {
  timestamp: "2026-10-04T01:00:00Z",
  level: "INFO",
  fields: { event: "process_started", error_id: "id", operation: "runtime_start" },
};

test("queries combine level, operation, event, identity, time and literal text", () => {
  assert.ok(
    matchesLog(event, {
      level: "info",
      operation: "runtime_start",
      event: "process_started",
      errorId: "id",
      since: "2026-10-03",
      contains: "process_started",
    }),
  );
  for (const options of [
    { level: "WARN" },
    { operation: "other" },
    { event: "exit" },
    { errorId: "other" },
    { since: "2026-10-05" },
    { contains: "absent" },
  ])
    assert.equal(matchesLog(event, options), false);
  assert.equal(matchesLog(null), false);
  assert.equal(matchesLog({}), false);
  assert.ok(matchesLog({ ...event, span: { name: "start" } }, { operation: "start" }));
});
test("query streams JSON Lines and counts malformed records", async () => {
  const directory = await mkdtemp(join(tmpdir(), "runtime-query-"));
  try {
    const path = join(directory, "runtime.jsonl");
    await writeFile(
      path,
      `${JSON.stringify(event)}\nnot-json\n\n${JSON.stringify({ ...event, level: "ERROR" })}\n`,
    );
    let output = "";
    const result = await queryLog(
      path,
      { level: "INFO" },
      {
        write: (value) => {
          output += value;
        },
      },
    );
    assert.deepEqual(result, { matched: 1, malformed: 1 });
    assert.deepEqual(JSON.parse(output), event);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
test("invalid CLI filters fail before reading files", () => {
  assert.throws(() => parseQuery([]));
  assert.throws(() => parseQuery(["file", "--unknown", "value"]));
  assert.throws(() => parseQuery(["file", "--level", "INVALID"]));
  assert.throws(() => parseQuery(["file", "--since", "not-time"]));
  assert.deepEqual(parseQuery(["file", "--level", "INFO"]), {
    path: "file",
    options: { level: "INFO" },
  });
});
