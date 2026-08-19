export interface AuthSession {
  accessToken: string;
  expiresAt: string;
}

const SESSION_KEY = "wms.web-admin.auth-session";

function storageAvailable(): boolean {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function isAuthSession(value: unknown): value is AuthSession {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return typeof record.accessToken === "string" && typeof record.expiresAt === "string";
}

export function readAuthSession(): AuthSession | null {
  if (!storageAvailable()) {
    return null;
  }
  const raw = window.localStorage.getItem(SESSION_KEY);
  if (!raw) {
    return null;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!isAuthSession(parsed) || !parsed.accessToken.trim()) {
      clearAuthSession();
      return null;
    }
    if (Number.isNaN(Date.parse(parsed.expiresAt)) || Date.parse(parsed.expiresAt) <= Date.now()) {
      clearAuthSession();
      return null;
    }
    return parsed;
  } catch {
    clearAuthSession();
    return null;
  }
}

export function readAccessToken(): string | null {
  return readAuthSession()?.accessToken ?? null;
}

export function hasActiveAuthSession(): boolean {
  return readAuthSession() !== null;
}

export function saveAuthSession(session: AuthSession): void {
  if (!storageAvailable()) {
    return;
  }
  window.localStorage.setItem(SESSION_KEY, JSON.stringify(session));
}

export function clearAuthSession(): void {
  if (!storageAvailable()) {
    return;
  }
  window.localStorage.removeItem(SESSION_KEY);
}
