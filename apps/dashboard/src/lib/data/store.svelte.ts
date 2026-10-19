import type { ReportBundle } from "@elcode-forge/schema";
import type { DataSource } from "./source.js";

class DashboardState {
  bundle = $state<ReportBundle | null>(null);
  source = $state<DataSource | null>(null);
  error = $state<string | null>(null);
  loading = $state<boolean>(false);

  setBundle(bundle: ReportBundle, source: DataSource) {
    this.bundle = bundle;
    this.source = source;
    this.error = null;
    this.loading = false;
  }

  setError(error: string) {
    this.error = error;
    this.loading = false;
  }

  setLoading(loading: boolean) {
    this.loading = loading;
  }
}

export const dashboardState = new DashboardState();
