import axios, { AxiosError } from "axios";

export interface ApiFieldError {
  field: string;
  code: string;
  message: string;
}

interface ErrorEnvelope {
  code?: string;
  message?: string;
  status?: number;
  fields?: ApiFieldError[];
  error?: string;
}

const STATUS_FALLBACK: Record<number, string> = {
  400: "Some of the details look wrong. Please check and try again.",
  401: "Your session has expired. Please sign in again.",
  403: "You don't have access to this.",
  404: "We couldn't find what you were looking for.",
  408: "The request took too long. Try again.",
  409: "That conflicts with the current state. Refresh and retry.",
  422: "Some of the details look wrong. Please check and try again.",
  423: "This is locked. Try again later.",
  429: "Too many requests. Please slow down.",
  500: "Something went wrong on our side. Please try again.",
  502: "We're having trouble reaching the server. Try again shortly.",
  503: "Service is temporarily unavailable. Try again shortly.",
  504: "The server is taking too long to respond. Try again.",
};

const CODE_FALLBACK: Record<string, string> = {
  EMAIL_ALREADY_TAKEN: "An account with this email already exists.",
  INVALID_CREDENTIALS: "Wrong email or password.",
  EMAIL_NOT_VERIFIED: "Please verify your email to continue.",
  VERIFICATION_CODE_EXPIRED:
    "The verification code has expired. Request a new one.",
  VERIFICATION_CODE_INVALID: "That verification code is invalid.",
  REFRESH_TOKEN_REVOKED: "Your session has been revoked. Please sign in again.",
  SESSION_EXPIRED: "Your session has expired. Please sign in again.",
  RESOURCE_NOT_FOUND: "We couldn't find what you were looking for.",
  RESOURCE_ALREADY_EXISTS: "That already exists.",
  CHANNEL_NOT_OWNER: "Only the channel owner can do that.",
  SERVER_NOT_OWNER: "Only the server owner can do that.",
  INVALID_CHANNEL_PASSWORD: "Wrong channel password.",
  VALIDATION_FAILED: "Some of the details look wrong. Please check and try again.",
  SERVICE_UNAVAILABLE: "Service is temporarily unavailable. Try again shortly.",
};

function envelope(error: unknown): ErrorEnvelope | undefined {
  if (!axios.isAxiosError(error)) return undefined;
  const data = (error as AxiosError<unknown>).response?.data;
  if (data && typeof data === "object") return data as ErrorEnvelope;
  return undefined;
}

export function getApiErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const ax = error as AxiosError<ErrorEnvelope>;

    if (ax.code === "ERR_NETWORK" || ax.message === "Network Error") {
      return "Can't reach the server. Check your internet connection.";
    }
    if (ax.code === "ECONNABORTED") {
      return "The request took too long. Try again.";
    }

    const body = envelope(error);
    if (body?.code && CODE_FALLBACK[body.code]) {
      return CODE_FALLBACK[body.code];
    }
    if (body?.message && body.message.trim().length > 0) {
      return body.message;
    }
    if (body?.error && body.error.trim().length > 0) {
      return body.error;
    }

    const status = ax.response?.status;
    if (status && STATUS_FALLBACK[status]) {
      return STATUS_FALLBACK[status];
    }
  }

  if (error instanceof Error && error.message) {
    if (/^Request failed with status/i.test(error.message)) {
      return "Something went wrong. Please try again.";
    }
    return error.message;
  }

  return "Something went wrong. Please try again.";
}

export class ApiError extends Error {
  status?: number;
  code?: string;
  requestId?: string;
  fields?: ApiFieldError[];

  constructor(
    message: string,
    status?: number,
    code?: string,
    requestId?: string,
    fields?: ApiFieldError[],
  ) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.requestId = requestId;
    this.fields = fields;
  }
}

function headerRequestId(error: unknown): string | undefined {
  if (!axios.isAxiosError(error)) return undefined;
  const headers = (error as AxiosError).response?.headers as
    | Record<string, string | string[]>
    | undefined;
  if (!headers) return undefined;
  const value = headers["x-request-id"] ?? headers["X-Request-Id"];
  if (Array.isArray(value)) return value[0];
  return typeof value === "string" ? value : undefined;
}

export function toApiError(error: unknown): Error {
  if (error instanceof ApiError) return error;
  if (error instanceof Error && !axios.isAxiosError(error)) return error;
  const status = axios.isAxiosError(error) ? error.response?.status : undefined;
  const body = envelope(error);
  return new ApiError(
    getApiErrorMessage(error),
    status,
    body?.code,
    headerRequestId(error),
    body?.fields,
  );
}
