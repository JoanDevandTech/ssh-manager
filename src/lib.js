export const quote = (v) => (/[\s"']/.test(v) ? `"${String(v).replace(/"/g, '\\"')}"` : String(v));

export function buildCommand(s) {
  const parts = ['ssh'];
  if (s.keyPath) parts.push('-i', quote(s.keyPath));
  if (s.port && Number(s.port) !== 22) parts.push('-p', String(s.port));
  parts.push(s.user ? `${s.user}@${s.host}` : s.host);
  return parts.join(' ');
}

// Rewrite a path under the home dir to ~/… (normalizes \ to /).
export function shortenHome(path, home) {
  const p = String(path).replace(/\\/g, '/');
  const h = String(home).replace(/\\/g, '/').replace(/\/+$/, '');
  return p === h || p.startsWith(h + '/') ? '~' + p.slice(h.length) : p;
}

export function filterSessions(sessions, query) {
  const q = String(query || '').trim().toLowerCase();
  if (!q) return [...sessions];
  return sessions.filter((s) => [s.name, s.host, s.user].some((v) => (v || '').toLowerCase().includes(q)));
}
