function assetBase(): string {
  const electron = (window as unknown as { electron?: { appBase?: string } })
    .electron;
  if (electron?.appBase) return electron.appBase;
  const pub = process.env.PUBLIC_URL || ".";
  const baseStr = pub.endsWith("/") ? pub : pub + "/";
  return new URL(baseStr, window.location.origin + "/").toString();
}

export function assetUrl(rel: string): string {
  return new URL(rel, assetBase()).toString();
}

export async function fetchAssetText(rel: string): Promise<string> {
  const response = await fetch(assetUrl(rel));
  if (!response.ok) throw new Error(`Asset ${rel} is unavailable (${response.status})`);
  return response.text();
}
