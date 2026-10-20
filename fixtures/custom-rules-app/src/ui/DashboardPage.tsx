import React from "react";
import { getDbConnection } from "../db/connection";

export function DashboardPage() {
  const conn = getDbConnection();
  return <div>Dashboard DB: {conn}</div>;
}
