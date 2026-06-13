export interface Session {
  userId: string;
}

const store = new Map<string, Session>();

export function loadSession(id: string): Session | null {
  return store.get(id) ?? null;
}
