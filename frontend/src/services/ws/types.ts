import type { WsEnvelope } from "../websocket";

export type WsHandler = (
  data: Record<string, unknown>,
  env: WsEnvelope,
) => void;

export type WsHandlers = Record<string, WsHandler>;
