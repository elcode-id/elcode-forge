import { queryDb } from '../db/client';

export interface UserRecord {
  id: string;
  name: string;
}

export function getActiveUsers(): UserRecord[] {
  return [queryDb('SELECT id, name FROM users LIMIT 1')];
}

export function formatUserAuditTag(id: string): string {
  return `audit:${id}`;
}
