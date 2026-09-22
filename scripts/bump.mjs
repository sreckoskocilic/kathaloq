import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? "")) {
  console.error("usage: npm run bump -- <x.y.z>");
  process.exit(1);
}

function replace(path, pattern, replacement) {
  const before = readFileSync(path, "utf8");
  const after = before.replace(pattern, replacement);
  if (after === before && !before.includes(version)) {
    console.error(`no version field matched in ${path}`);
    process.exit(1);
  }
  writeFileSync(path, after);
  console.log(`${path} -> ${version}`);
}

execFileSync("npm", ["version", version, "--no-git-tag-version", "--allow-same-version"], {
  stdio: "inherit",
});
replace("src-tauri/tauri.conf.json", /("version":\s*")[^"]+(")/, `$1${version}$2`);
replace("src-tauri/Cargo.toml", /^(version = ")[^"]+(")/m, `$1${version}$2`);
replace("src-tauri/Cargo.lock", /(name = "kathaloq"\nversion = ")[^"]+(")/, `$1${version}$2`);
