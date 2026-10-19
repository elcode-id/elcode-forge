import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { compile } from "json-schema-to-typescript";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const SCHEMA_PATH = path.resolve(__dirname, "../schema/report-bundle.schema.json");
const OUTPUT_PATH = path.resolve(__dirname, "../src/generated.ts");

async function main() {
  const schemaContent = await fs.readFile(SCHEMA_PATH, "utf-8");
  const schema = JSON.parse(schemaContent);

  const ts = await compile(schema, "ReportBundle", {
    bannerComment:
      "/* eslint-disable */\n/**\n * This file was automatically generated from report-bundle.schema.json.\n * DO NOT MODIFY IT BY HAND. Instead, modify the Rust IR structs and run `eforge schema`.\n */",
    style: {
      singleQuote: false,
      semi: true,
    },
  });

  await fs.mkdir(path.dirname(OUTPUT_PATH), { recursive: true });
  await fs.writeFile(OUTPUT_PATH, ts, "utf-8");
  console.log(`Generated TypeScript types at: ${OUTPUT_PATH}`);
}

main().catch((err) => {
  console.error("Failed to generate TypeScript types:", err);
  process.exit(1);
});
