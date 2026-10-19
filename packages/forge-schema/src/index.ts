export type {
  ArchitectureSummary,
  ChangeImpactReport,
  Confidence,
  Dependency,
  DimensionScore,
  FileChange,
  FileNode,
  Finding,
  GitRevision,
  HealthScore,
  HotspotEntry,
  Module,
  ReportBundle,
  Repository,
  RiskLevel,
  RiskReport,
  Severity,
  SourceLocation,
} from "./generated";

export {
  SUPPORTED_SCHEMA_VERSION,
  parseReportBundle,
  type ParseReportBundleResult,
} from "./validate";
