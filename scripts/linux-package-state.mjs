import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { DatabaseSync } from "node:sqlite";

// Only used with the disposable XDG directory in verify-linux-packages.sh.
const [operation, databasePath, snapshotPath] = process.argv.slice(2);
assert.ok(databasePath && snapshotPath, "需要数据库及验收快照路径");
const database = new DatabaseSync(databasePath);
try {
  assert.equal(database.prepare("PRAGMA integrity_check").get().integrity_check, "ok");
  if (operation === "seed") {
    const configuration = JSON.parse(
      await readFile(
        new URL("./fixtures/linux-upgrade-configuration.json", import.meta.url),
        "utf8",
      ),
    );
    database
      .prepare(
        "INSERT INTO configuration VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET document_json=excluded.document_json",
      )
      .run(JSON.stringify(configuration));
    database
      .prepare(
        "INSERT INTO selected_mode VALUES (1, 'direct') ON CONFLICT(id) DO UPDATE SET mode=excluded.mode",
      )
      .run();
  }
  const state = Object.fromEntries(
    ["configuration", "selected_mode"].map((table) => [
      table,
      database.prepare(`SELECT * FROM ${table} ORDER BY id`).all(),
    ]),
  );
  assert.equal(state.configuration.length, 1, "必须包含非空代理配置");
  if (operation === "seed" || operation === "snapshot") {
    await writeFile(snapshotPath, JSON.stringify(state));
  } else {
    assert.equal(operation, "verify");
    assert.deepEqual(
      JSON.parse(JSON.stringify(state)),
      JSON.parse(await readFile(snapshotPath, "utf8")),
    );
  }
  console.log(`${operation}: configuration and selected mode verified`);
} finally {
  database.close();
}
