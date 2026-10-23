/**
 * Client-side Software Bill of Materials (SBOM) generator and exporter.
 * Produces valid CycloneDX v1.5 and SPDX v2.3 JSON documents directly from ReportBundle.
 */

import type { Dependency, ReportBundle } from "@elcode-forge/schema";

interface SpdxPackage {
  SPDXID: string;
  name: string;
  versionInfo: string;
  downloadLocation: string;
  filesAnalyzed: boolean;
  primaryPackagePurpose: string;
  externalRefs?: Array<{
    referenceCategory: string;
    referenceType: string;
    referenceLocator: string;
  }>;
}

/**
 * Generates canonical Package URL (PURL) for a given dependency.
 */
export function generatePurl(dep: Dependency): string {
  const eco = (dep.ecosystem || "generic").toLowerCase();
  const rawVersion = dep.resolved_version || dep.version_req || "0.0.0";
  const cleanVersion = rawVersion.replace(/^[=^~><\s]+/, "") || "0.0.0";

  switch (eco) {
    case "npm": {
      if (dep.name.includes("/")) {
        const [scope, name] = dep.name.split("/");
        const cleanScope = scope.startsWith("@") ? scope.slice(1) : scope;
        return `pkg:npm/%40${cleanScope}/${name}@${cleanVersion}`;
      }
      return `pkg:npm/${dep.name}@${cleanVersion}`;
    }
    case "pypi": {
      const normalized = dep.name.toLowerCase().replace(/_/g, "-");
      return `pkg:pypi/${normalized}@${cleanVersion}`;
    }
    case "go":
      return `pkg:golang/${dep.name}@${cleanVersion}`;
    case "cargo":
      return `pkg:cargo/${dep.name}@${cleanVersion}`;
    case "composer":
      return `pkg:composer/${dep.name}@${cleanVersion}`;
    default:
      return `pkg:generic/${dep.name}@${cleanVersion}`;
  }
}

/**
 * Generates an OASIS / OWASP CycloneDX v1.5 JSON document string.
 */
export function generateClientCycloneDx(bundle: ReportBundle): string {
  const projectName = bundle.repository?.name || "project";
  const dependencies = bundle.repository?.dependencies || [];

  const components = dependencies.map((dep) => {
    const rawVersion = dep.resolved_version || dep.version_req || "0.0.0";
    const cleanVersion = rawVersion.replace(/^[=^~><\s]+/, "") || "0.0.0";
    return {
      type: "library",
      name: dep.name,
      version: cleanVersion,
      purl: generatePurl(dep),
      scope: dep.is_dev ? "optional" : "required",
      properties: [
        { name: "forge:ecosystem", value: dep.ecosystem },
        { name: "forge:is_direct", value: String(dep.is_direct) },
      ],
    };
  });

  const doc = {
    $schema: "http://cyclonedx.org/schema/bom-1.5.schema.json",
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    serialNumber: `urn:uuid:${Math.random().toString(36).substring(2)}-${Date.now()}`,
    version: 1,
    metadata: {
      tools: [
        {
          vendor: "ELCODE",
          name: "eforge",
          version: bundle.tool_version || "0.9.0",
        },
      ],
      component: {
        type: "application",
        name: projectName,
        version: "1.0.0",
      },
    },
    components,
  };

  return JSON.stringify(doc, null, 2);
}

/**
 * Generates an ISO/IEC 5962:2021 SPDX v2.3 JSON document string.
 */
export function generateClientSpdx(bundle: ReportBundle): string {
  const projectName = bundle.repository?.name || "project";
  const dependencies = bundle.repository?.dependencies || [];

  const packages: SpdxPackage[] = [
    {
      SPDXID: "SPDXRef-RootPackage",
      name: projectName,
      versionInfo: "1.0.0",
      downloadLocation: "NOASSERTION",
      filesAnalyzed: false,
      primaryPackagePurpose: "APPLICATION",
    },
  ];

  const relationships = [
    {
      spdxElementId: "SPDXRef-DOCUMENT",
      relationshipType: "DESCRIBES",
      relatedSpdxElement: "SPDXRef-RootPackage",
    },
  ];

  dependencies.forEach((dep, idx) => {
    const rawVersion = dep.resolved_version || dep.version_req || "0.0.0";
    const cleanVersion = rawVersion.replace(/^[=^~><\s]+/, "") || "0.0.0";
    const spdxId = `SPDXRef-Package-${idx + 1}`;

    packages.push({
      SPDXID: spdxId,
      name: dep.name,
      versionInfo: cleanVersion,
      downloadLocation: "NOASSERTION",
      filesAnalyzed: false,
      primaryPackagePurpose: "LIBRARY",
      externalRefs: [
        {
          referenceCategory: "PACKAGE-MANAGER",
          referenceType: "purl",
          referenceLocator: generatePurl(dep),
        },
      ],
    });

    relationships.push({
      spdxElementId: "SPDXRef-RootPackage",
      relationshipType: "DEPENDS_ON",
      relatedSpdxElement: spdxId,
    });
  });

  const doc = {
    spdxVersion: "SPDX-2.3",
    dataLicense: "CC0-1.0",
    SPDXID: "SPDXRef-DOCUMENT",
    name: projectName,
    documentNamespace: `https://spdx.org/spdxdocs/${projectName}-${Date.now()}`,
    creationInfo: {
      creators: [
        `Tool: elcode-forge-${bundle.tool_version || "0.9.0"}`,
        "Organization: ELCODE",
      ],
      created: "2024-01-01T00:00:00Z",
    },
    packages,
    relationships,
  };

  return JSON.stringify(doc, null, 2);
}

/**
 * Triggers a browser download of a text/JSON file.
 */
export function downloadFile(
  filename: string,
  content: string,
  mimeType = "application/json"
): void {
  if (typeof window === "undefined" || typeof document === "undefined") return;

  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
