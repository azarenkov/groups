export type ScreenContent = "auto" | "text" | "motion";

export const SCREEN_CODEC = "vp9" as const;

export function contentHintFor(
  content: ScreenContent,
  frameRate: number,
): "detail" | "motion" {
  if (content === "text") return "detail";
  if (content === "motion") return "motion";
  return frameRate >= 60 ? "motion" : "detail";
}
