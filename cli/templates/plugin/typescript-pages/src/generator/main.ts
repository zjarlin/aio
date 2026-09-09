import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

import { definitionJson } from "../pages/definition.js";

const output = resolve(process.argv[2] ?? "dist/pages.json");
await mkdir(dirname(output), { recursive: true });
await writeFile(output, `${definitionJson()}\n`, "utf8");
process.stdout.write(`${output}\n`);
