import { publicAppOrigin } from "../../../shared/config/runtime";
import { copyTextToClipboard } from "../../../shared/lib/clipboard";

export const profileLink = (userId: string): string => `${publicAppOrigin()}/u/${userId}`;

export const prefersNativeShare = (): boolean =>
  typeof navigator.share === "function" &&
  typeof window.matchMedia === "function" &&
  window.matchMedia("(max-width: 640px)").matches;

export type ShareOutcome = "shared" | "copied" | "cancelled";

export async function shareProfile(userId: string, name?: string): Promise<ShareOutcome> {
  const url = profileLink(userId);
  if (prefersNativeShare()) {
    try {
      await navigator.share({ url, title: name ? `@${name}` : "Rift" });
      return "shared";
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return "cancelled";
    }
  }
  await copyTextToClipboard(url);
  return "copied";
}
