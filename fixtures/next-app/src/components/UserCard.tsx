import React from 'react';
// Intentional architectural boundary violation: UI component directly imports database client
import { queryDb } from '../db/client';

export interface UserCardProps {
  userId: string;
}

export function UserCard({ userId }: UserCardProps) {
  const row = queryDb(`SELECT id, name FROM users WHERE id = '${userId}'`);
  return <div className="user-card">{row.name}</div>;
}
