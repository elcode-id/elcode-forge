// Intentional circular dependency with services/userService for SCC cycle detection tests
import { formatUserAuditTag } from '../services/userService';

export function queryDb(sql: string): { id: string; name: string; tag: string } {
  return {
    id: 'usr_01',
    name: 'Fixture User',
    tag: formatUserAuditTag(sql),
  };
}
