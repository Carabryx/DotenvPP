import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const outputDir = process.argv[2] ?? "pkg";
const packagePath = join(outputDir, "package.json");
const raw = await readFile(packagePath, "utf8");
const pkg = JSON.parse(raw);

pkg.name = "@dotenvpp/wasm";
pkg.exports = {
  ".": {
    types: "./dotenvpp_wasm.d.ts",
    default: "./dotenvpp_wasm.js"
  },
  "./wasm": "./dotenvpp_wasm_bg.wasm"
};
pkg.keywords = ["dotenv", "env", "config", "wasm", "schema"];

await writeFile(packagePath, `${JSON.stringify(pkg, null, 2)}\n`);
