export function allowed(user: { role: string }): boolean {
  return user.role === 'member' || user.role === 'admin';
}
