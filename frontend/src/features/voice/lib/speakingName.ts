export function speakingName(
  participants: Record<string, string>,
  speakingIds: Set<string>,
  selfId: string | null,
): string | null {
  for (const [id, name] of Object.entries(participants)) {
    if (id === selfId) continue;
    if (speakingIds.has(id)) return name || id;
  }
  return null;
}
