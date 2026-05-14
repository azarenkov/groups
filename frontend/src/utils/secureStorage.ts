const LS_KEY = "rift-refresh-fallback";

function api() {
  return typeof window !== "undefined" ? window.electron : undefined;
}

export async function getRefreshToken(): Promise<string | null> {
  const electron = api();
  if (electron?.secureGet) {
    try {
      const value = await electron.secureGet();
      if (value) return value;
    } catch {}
  }
  return localStorage.getItem(LS_KEY);
}

export async function setRefreshToken(value: string): Promise<void> {
  const electron = api();
  if (electron?.secureSet) {
    const ok = await electron.secureSet(value);
    if (!ok) localStorage.setItem(LS_KEY, value);
    return;
  }
  localStorage.setItem(LS_KEY, value);
}

export async function clearRefreshToken(): Promise<void> {
  const electron = api();
  if (electron?.secureClear) {
    await electron.secureClear();
  }
  localStorage.removeItem(LS_KEY);
}
