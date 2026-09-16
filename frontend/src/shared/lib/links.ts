export const URL_REGEX = /https?:\/\/[^\s<>"')\]}]+[^\s<>"')\]}.,;:!?]/g;

export interface ParsedLink {
  url: string;
  kind: "image" | "youtube" | "generic";
  youtubeId?: string;
}

const IMAGE_EXT = /\.(png|jpe?g|gif|webp|avif|svg)$/i;

export function youtubeIdFrom(url: string): string | null {
  try {
    const u = new URL(url);
    const host = u.hostname.replace(/^www\./, "");
    if (host === "youtu.be") {
      const id = u.pathname.slice(1).split("/")[0];
      return id.length === 11 ? id : null;
    }
    if (host === "youtube.com" || host === "m.youtube.com") {
      if (u.pathname === "/watch") {
        const id = u.searchParams.get("v");
        return id && id.length === 11 ? id : null;
      }
      const short = u.pathname.match(/^\/(shorts|embed)\/([\w-]{11})/);
      if (short) return short[2];
    }
    return null;
  } catch {
    return null;
  }
}

export function extractLinks(text: string): ParsedLink[] {
  const out: ParsedLink[] = [];
  const seen = new Set<string>();
  for (const match of text.matchAll(URL_REGEX)) {
    const url = match[0];
    if (seen.has(url)) continue;
    seen.add(url);
    const youtubeId = youtubeIdFrom(url);
    let pathname = "";
    try {
      pathname = new URL(url).pathname;
    } catch {
      continue;
    }
    out.push({
      url,
      kind: youtubeId ? "youtube" : IMAGE_EXT.test(pathname) ? "image" : "generic",
      youtubeId: youtubeId ?? undefined,
    });
  }
  return out;
}

export function domainOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
