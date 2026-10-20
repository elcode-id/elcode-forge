import { getDbConnection } from "../db/connection.js";

export class UserService {
  getUser(id: string) {
    const db = getDbConnection();
    return { id, db };
  }
}
