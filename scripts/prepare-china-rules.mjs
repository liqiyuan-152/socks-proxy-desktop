import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { copyFileSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { readFile, writeFile, mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { isIP } from "node:net";

const domainCommit = "c1c2cf0d252871e8739747714df06e8d2671f72f";
const ipCommit = "8046f18143a4ad0ced43fe5d7e72e0c68ba73ac8";
const ipLicenseCommit = "b2b8edc0ddd9cf2bc638ec77587301bbf84b8db9";
const sources = {
  domains: {
    url: `https://github.com/v2fly/domain-list-community/archive/${domainCommit}.zip`,
    sha256: "b43a766bef9fff982bd3ed822f7f9d876ed61d9d3bb8650d09639f38cda7b2f6",
  },
  ip: {
    url: `https://raw.githubusercontent.com/gaoyifan/china-operator-ip/${ipCommit}/china46.txt`,
    sha256: "3c321e1afb24dee357dd77eade0ed2d91f411547386d759550ace084505dfcaf",
  },
  ipLicense: {
    url: `https://raw.githubusercontent.com/gaoyifan/china-operator-ip/${ipLicenseCommit}/LICENSE`,
    sha256: "8f4d03a581484143945a9fac471c3b669d5aada68361fd1f25886c3ddbdd786a",
  },
};
const outputDir = resolve("src-tauri/resources/china-rules");
const core = resolve("src-tauri/resources/sing-box/windows-amd64/sing-box.exe");
const sourceDir = process.argv[2] === "--source-dir" ? resolve(process.argv[3]) : null;

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function download(source, filename) {
  let bytes;
  if (sourceDir) {
    bytes = await readFile(join(sourceDir, filename));
  } else {
    const response = await fetch(source.url);
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${source.url}`);
    bytes = Buffer.from(await response.arrayBuffer());
  }
  if (digest(bytes) !== source.sha256) throw new Error(`SHA-256 mismatch: ${source.url}`);
  return bytes;
}

function parseLine(raw, source) {
  const line = raw.split("#", 1)[0].trim();
  if (!line) return null;
  const [item, ...modifiers] = line.split(/\s+/);
  const [prefix, value] = item.includes(":") ? item.split(/:(.*)/s, 2) : ["domain", item];
  if (!value || !["domain", "full", "keyword", "regexp", "include"].includes(prefix)) {
    throw new Error(`Unsupported domain entry in ${source}: ${raw}`);
  }
  for (const modifier of modifiers) {
    if (!/^[@&][\w!-]+$/.test(modifier)) {
      throw new Error(`Unsupported domain modifier in ${source}: ${raw}`);
    }
  }
  return { prefix, value, attrs: modifiers.filter((part) => part.startsWith("@")) };
}

async function expandDomains(dir, name, stack = []) {
  if (stack.includes(name) || !/^[a-z0-9!-]+$/.test(name)) {
    throw new Error(`Invalid or cyclic domain include: ${[...stack, name].join(" -> ")}`);
  }
  const lines = (await readFile(join(dir, "data", name), "utf8")).split(/\r?\n/);
  const entries = await Promise.all(
    lines.map(async (raw) => {
      const item = parseLine(raw, name);
      if (!item) return [];
      if (item.prefix !== "include") {
        return [item];
      }
      const included = await expandDomains(dir, item.value, [...stack, name]);
      return included.filter(({ attrs }) =>
        item.attrs.every((filter) => {
          const attr = filter.slice(1);
          return attr.startsWith("-")
            ? !attrs.includes(`@${attr.slice(1)}`)
            : attrs.includes(filter);
        }),
      );
    }),
  );
  return entries.flat();
}

function ruleSet(field, values) {
  if (!values.length) throw new Error(`Empty rule set: ${field}`);
  return { version: 3, rules: [{ [field]: [...new Set(values)].sort() }] };
}

function runCore(args) {
  const result = spawnSync(core, args, { encoding: "utf8" });
  if (result.status !== 0)
    throw new Error(`sing-box ${args.join(" ")}: ${result.stderr || result.error}`);
}

if (process.platform !== "win32") throw new Error("The pinned sing-box core is Windows-only");
const temp = await mkdtemp(join(tmpdir(), "socks-proxy-china-rules-"));
try {
  const [domainZip, ipText, ipLicense] = await Promise.all([
    download(sources.domains, "domains.zip"),
    download(sources.ip, "china46.txt"),
    download(sources.ipLicense, "ip-LICENSE"),
  ]);
  const archive = join(temp, "domains.zip");
  await writeFile(archive, domainZip);
  const extracted = spawnSync("tar", ["-xf", archive, "-C", temp], { encoding: "utf8" });
  if (extracted.status !== 0) throw new Error(`Cannot extract domain source: ${extracted.stderr}`);
  const domainRoot = join(temp, `domain-list-community-${domainCommit}`);
  const domains = await expandDomains(domainRoot, "cn");
  const kinds = { domain_suffix: [], domain: [], domain_keyword: [], domain_regex: [] };
  const fields = {
    domain: "domain_suffix",
    full: "domain",
    keyword: "domain_keyword",
    regexp: "domain_regex",
  };
  for (const entry of domains) kinds[fields[entry.prefix]].push(entry.value);
  const cidrs = { ipv4: [], ipv6: [] };
  for (const raw of ipText.toString("utf8").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const [address, bits, extra] = line.split("/");
    const family = isIP(address);
    if (extra || !family || !/^\d+$/.test(bits) || Number(bits) > (family === 4 ? 32 : 128)) {
      throw new Error(`Invalid source CIDR: ${line}`);
    }
    cidrs[family === 4 ? "ipv4" : "ipv6"].push(line);
  }
  const payloads = {
    "china-domains": {
      version: 3,
      rules: [
        Object.fromEntries(
          Object.entries(kinds)
            .filter(([, values]) => values.length)
            .map(([key, values]) => [key, [...new Set(values)].sort()]),
        ),
      ],
    },
    "china-ipv4": ruleSet("ip_cidr", cidrs.ipv4),
    "china-ipv6": ruleSet("ip_cidr", cidrs.ipv6),
  };
  const hashes = {};
  for (const [name, payload] of Object.entries(payloads)) {
    const json = join(temp, `${name}.json`);
    const binary = join(temp, `${name}.srs`);
    writeFileSync(json, JSON.stringify(payload));
    runCore(["rule-set", "compile", json, "-o", binary]);
    hashes[`${name}.srs`] = digest(readFileSync(binary));
  }
  await mkdir(outputDir, { recursive: true });
  for (const name of Object.keys(payloads)) {
    const destination = join(outputDir, `${name}.srs`);
    const staged = `${destination}.pending`;
    copyFileSync(join(temp, `${name}.srs`), staged);
    renameSync(staged, destination);
  }
  const domainLicense = await readFile(join(domainRoot, "LICENSE"));
  await writeFile(join(outputDir, "LICENSE.domain-list-community"), domainLicense);
  await writeFile(join(outputDir, "LICENSE.china-operator-ip"), ipLicense);
  const manifest = {
    data_date: "2026-09-23",
    core_version: "1.14.1",
    sources: {
      domain: {
        repository: "v2fly/domain-list-community",
        commit: domainCommit,
        license: "MIT",
        license_sha256: digest(domainLicense),
        ...sources.domains,
      },
      ip: {
        repository: "gaoyifan/china-operator-ip",
        commit: ipCommit,
        license: "MIT",
        license_commit: ipLicenseCommit,
        license_sha256: digest(ipLicense),
        ...sources.ip,
      },
    },
    hashes,
    counts: Object.fromEntries(
      Object.entries(kinds).map(([key, values]) => [key, new Set(values).size]),
    ),
    ip_counts: { ipv4: cidrs.ipv4.length, ipv6: cidrs.ipv6.length },
  };
  await writeFile(join(outputDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
  console.log(
    `Compiled China rule sets: ${JSON.stringify(manifest.counts)}, IPv4 ${cidrs.ipv4.length}, IPv6 ${cidrs.ipv6.length}`,
  );
} finally {
  await rm(temp, { recursive: true, force: true });
}
