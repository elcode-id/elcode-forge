import Ajv, { type ValidateFunction } from "ajv";
import schema from "../schema/report-bundle.schema.json";
import type { ReportBundle } from "./generated";

export const SUPPORTED_SCHEMA_VERSION = 1;

const ajv = new Ajv({
  allErrors: true,
  strict: false,
});

const validateSchema: ValidateFunction<ReportBundle> = ajv.compile<ReportBundle>(schema);

export type ParseReportBundleResult =
  | { ok: true; bundle: ReportBundle }
  | { ok: false; errors: string[] };

/**
 * Validates untrusted input against the deterministic ReportBundle JSON Schema
 * and verifies that the `schema_version` matches the supported version.
 */
export function parseReportBundle(input: unknown): ParseReportBundleResult {
  if (!input || typeof input !== "object") {
    return { ok: false, errors: ["Input must be a non-null object"] };
  }

  const valid = validateSchema(input);
  if (!valid) {
    const errors = (validateSchema.errors ?? []).map((err) => {
      const path = err.instancePath || "/";
      const message = err.message ?? "validation failed";
      return `${path}: ${message}`;
    });
    return { ok: false, errors: errors.length > 0 ? errors : ["Schema validation failed"] };
  }

  const candidate = input as ReportBundle;
  if (candidate.schema_version !== SUPPORTED_SCHEMA_VERSION) {
    return {
      ok: false,
      errors: [
        `Unsupported schema_version: received ${candidate.schema_version}, expected ${SUPPORTED_SCHEMA_VERSION}`,
      ],
    };
  }

  return { ok: true, bundle: candidate };
}
