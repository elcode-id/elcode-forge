import type { ReportBundle } from "@elcode-forge/schema";
import type { DataSource } from "./source.js";
import {
  loadServerConfig,
  saveServerConfig,
  type ServerConfig,
  type ProjectSummary,
} from "./server.js";

class DashboardState {
  bundle = $state<ReportBundle | null>(null);
  source = $state<DataSource | null>(null);
  error = $state<string | null>(null);
  loading = $state<boolean>(false);

  // Team Cloud Mode states
  serverConfig = $state<ServerConfig>(loadServerConfig());
  serverProjects = $state<ProjectSummary[]>([]);
  serverConnected = $state<boolean>(false);

  setBundle(bundle: ReportBundle, source: DataSource) {
    this.bundle = bundle;
    this.source = source;
    this.error = null;
    this.loading = false;
  }

  setError(error: string | null) {
    this.error = error;
    this.loading = false;
  }

  setLoading(loading: boolean) {
    this.loading = loading;
  }

  setServerConfig(cfg: ServerConfig) {
    this.serverConfig = cfg;
    saveServerConfig(cfg);
  }

  setMode(mode: "local" | "cloud") {
    this.serverConfig.mode = mode;
    saveServerConfig(this.serverConfig);
  }

  setServerProjects(projects: ProjectSummary[]) {
    this.serverProjects = projects;
  }

  setServerConnected(connected: boolean) {
    this.serverConnected = connected;
  }
}

export const dashboardState = new DashboardState();
