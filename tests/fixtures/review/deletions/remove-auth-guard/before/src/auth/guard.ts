export function requireRole(role: string, user: { roles: string[] }): void {
  if (!user.roles.includes(role)) {
    throw new Error("forbidden");
  }
}
