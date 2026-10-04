import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

/** Query already sanitized application JSON Lines; malformed records are skipped. */
export function matchesLog(record, options = {}) {
  if (!record || typeof record !== "object" || !record.fields) return false;
  if (options.level && record.level !== options.level.toUpperCase()) return false;
  if (
    options.operation &&
    record.fields.operation !== options.operation &&
    record.span?.name !== options.operation
  )
    return false;
  if (
    options.event &&
    record.fields.event !== options.event &&
    record.fields.stage !== options.event
  )
    return false;
  if (options.errorId && record.fields.error_id !== options.errorId) return false;
  if (
    options.since &&
    (!record.timestamp || Date.parse(record.timestamp) < Date.parse(options.since))
  )
    return false;
  if (options.contains && !JSON.stringify(record).includes(options.contains)) return false;
  return true;
}

export async function queryLog(path, options, output = process.stdout) {
  let malformed = 0;
  let matched = 0;
  const lines = createInterface({ input: createReadStream(path), crlfDelay: Infinity });
  for await (const line of lines) {
    if (!line.trim()) continue;
    try {
      const record = JSON.parse(line);
      if (matchesLog(record, options)) {
        output.write(`${JSON.stringify(record)}\n`);
        matched++;
      }
    } catch {
      malformed++;
    }
  }
  return { matched, malformed };
}

export function parseQuery(args) {
  const options = {};
  const keys = {
    "--level": "level",
    "--operation": "operation",
    "--event": "event",
    "--error-id": "errorId",
    "--since": "since",
    "--contains": "contains",
  };
  const path = args.shift();
  if (!path)
    throw new Error(
      "Usage: node scripts/query-runtime-log.mjs <jsonl> [--level INFO] [--operation name] [--event name] [--error-id id] [--since ISO] [--contains text]",
    );
  while (args.length) {
    const key = keys[args.shift()];
    const value = args.shift();
    if (!key || !value) throw new Error("Invalid query option");
    options[key] = value;
  }
  if (
    options.level &&
    !["TRACE", "DEBUG", "INFO", "WARN", "ERROR"].includes(options.level.toUpperCase())
  )
    throw new Error("Invalid log level");
  if (options.since && !Number.isFinite(Date.parse(options.since)))
    throw new Error("Invalid timestamp");
  return { path, options };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const { path, options } = parseQuery(process.argv.slice(2));
    const result = await queryLog(path, options);
    process.stderr.write(`Matched ${result.matched}; malformed ${result.malformed}\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
