import React from 'react';
import { UserCard } from '../components/UserCard';
import { getActiveUsers } from '../services/userService';

export default function HomePage() {
  const users = getActiveUsers();
  return (
    <main>
      <h1>ELCODE Forge Fixture — Next.js App</h1>
      {users.map((u) => (
        <UserCard key={u.id} userId={u.id} />
      ))}
    </main>
  );
}
