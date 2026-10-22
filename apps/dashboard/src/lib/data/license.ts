/**
 * License classification and badge styling for open-source compliance in the dashboard.
 */

export type LicenseCategory =
  | "Permissive"
  | "WeakCopyleft"
  | "StrongCopyleft"
  | "Proprietary"
  | "Unknown";

/**
 * Categorizes an SPDX license identifier into standard compliance tiers.
 */
export function categorizeLicense(raw?: string): LicenseCategory {
  if (!raw || !raw.trim()) return "Unknown";
  const trimmed = raw.trim();

  if (trimmed.includes(" OR ")) {
    const parts = trimmed.split(" OR ");
    const cats = parts.map((p) => categorizeLicense(p));
    if (cats.includes("Permissive")) return "Permissive";
    if (cats.includes("WeakCopyleft")) return "WeakCopyleft";
  }

  if (trimmed.includes(" AND ")) {
    const parts = trimmed.split(" AND ");
    const cats = parts.map((p) => categorizeLicense(p));
    if (cats.includes("StrongCopyleft")) return "StrongCopyleft";
    if (cats.includes("WeakCopyleft")) return "WeakCopyleft";
    if (cats.includes("Proprietary")) return "Proprietary";
    return "Permissive";
  }

  const upper = trimmed.toUpperCase();

  // 1. Weak Copyleft (must check before GPL)
  if (
    upper.startsWith("LGPL") ||
    upper.includes("LGPL") ||
    upper.startsWith("MPL") ||
    upper.startsWith("EPL") ||
    upper.startsWith("CDDL")
  ) {
    return "WeakCopyleft";
  }

  // 2. Strong Copyleft
  if (
    upper.startsWith("GPL") ||
    upper.startsWith("AGPL") ||
    upper.includes("GPL-2") ||
    upper.includes("GPL-3") ||
    upper.includes("AGPL-3") ||
    upper.startsWith("OSL") ||
    upper.startsWith("SSPL") ||
    upper.startsWith("EUPL")
  ) {
    return "StrongCopyleft";
  }

  // 3. Permissive
  if (
    upper === "MIT" ||
    upper.startsWith("APACHE") ||
    upper.startsWith("BSD") ||
    upper === "ISC" ||
    upper === "UNLICENSE" ||
    upper.startsWith("CC0") ||
    upper === "0BSD" ||
    upper === "ZLIB" ||
    upper === "WTFPL" ||
    upper.startsWith("BSL") ||
    upper === "BOOST"
  ) {
    return "Permissive";
  }

  // 4. Proprietary
  if (
    upper === "PROPRIETARY" ||
    upper === "UNLICENSED" ||
    upper.includes("ALL-RIGHTS-RESERVED") ||
    upper === "COMMERCIAL"
  ) {
    return "Proprietary";
  }

  return "Unknown";
}

/**
 * Returns Tailwind badge classes corresponding to a license compliance category.
 */
export function getLicenseBadgeClass(category: LicenseCategory): string {
  switch (category) {
    case "Permissive":
      return "bg-emerald-950/70 text-emerald-300 border-emerald-800/80";
    case "WeakCopyleft":
      return "bg-amber-950/70 text-amber-300 border-amber-800/80";
    case "StrongCopyleft":
      return "bg-rose-950/70 text-rose-300 border-rose-800/80";
    case "Proprietary":
      return "bg-purple-950/70 text-purple-300 border-purple-800/80";
    default:
      return "bg-slate-800 text-slate-400 border-slate-700";
  }
}
