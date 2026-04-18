import { readFileSync } from "node:fs";
import { checkPolicy, initSync, parse, validate, version } from "../pkg-web/dotenvpp_wasm.js";

initSync({
  module: readFileSync(new URL("../pkg-web/dotenvpp_wasm_bg.wasm", import.meta.url)),
});

const parsed = JSON.parse(parse("PORT=8080\nLOG_LEVEL=info\n"));
if (parsed.length !== 2 || parsed[0].key !== "PORT") {
  throw new Error("parse failed");
}

const schema = `[vars.PORT]
type = "port"
required = true
`;
const validation = JSON.parse(validate("PORT=8080\n", schema));
if (validation.diagnostics.length !== 0) {
  throw new Error("validate failed");
}

const policy = `[[rules]]
name = "no-debug"
condition = "LOG_LEVEL == 'debug'"
severity = "error"
`;
const report = JSON.parse(checkPolicy("LOG_LEVEL=info\n", policy));
if (report.violations.length !== 0) {
  throw new Error("policy failed");
}

console.log(
  JSON.stringify({
    version: version(),
    parsed: parsed.length,
    diagnostics: validation.diagnostics.length,
    violations: report.violations.length,
  }),
);
