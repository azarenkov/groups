import { http } from "../../../shared/api/http";

export type ForwardSourceRef =
  | { kind: "channel"; communityId: string; channelId: string; messageId: string }
  | { kind: "dm"; friendId: string; messageId: string };

export type ForwardTarget =
  | { kind: "channel"; communityId: string; channelId: string }
  | { kind: "dm"; friendId: string };

export interface ForwardResult {
  target: ForwardTarget;
  messageId?: string;
  error?: { code: string; message: string };
}

export async function forwardMessage(
  source: ForwardSourceRef,
  targets: ForwardTarget[],
  comment?: string,
): Promise<ForwardResult[]> {
  const response = await http.post("/v1/users/me/forward", {
    source,
    targets,
    comment: comment && comment.trim().length > 0 ? comment : undefined,
  });
  return response.data.data.results;
}
