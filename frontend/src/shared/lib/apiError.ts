import axios, { AxiosError } from "axios";

import i18n from "../../i18n";

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

const NS = "errors";
const SECONDS_PATTERN = /(\d+)\s*seconds?\b/i;
const SIZE_LIMIT_PATTERN = /max\s+(\d+)\s*MB/i;

function translate(key: string, options?: Record<string, unknown>): string | undefined {
  if (!i18n.exists(key, { ns: NS })) return undefined;
  const value = i18n.t(key, { ns: NS, ...options });
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function nonEmpty(value: string | undefined): string | undefined {
  return value && value.trim().length > 0 ? value : undefined;
}

export interface ApiErrorDetails {
  code?: string;
  status?: number;
  message?: string;
  fields?: ApiFieldError[];
  retryAfterSeconds?: number;
}

function secondsFrom(details: ApiErrorDetails): number | undefined {
  if (details.retryAfterSeconds && details.retryAfterSeconds > 0) {
    return Math.ceil(details.retryAfterSeconds);
  }
  const match = details.message?.match(SECONDS_PATTERN);
  return match ? Number(match[1]) : undefined;
}

function translateCode(details: ApiErrorDetails): string | undefined {
  const { code } = details;
  if (!code) return undefined;
  if (code === "VALIDATION_FAILED") {
    for (const field of details.fields ?? []) {
      const text = translate(`fields.${field.code}`);
      if (text) return text;
    }
  }
  const seconds = secondsFrom(details);
  if (seconds !== undefined) {
    const timed = translate(`codes.${code}_seconds`, { seconds });
    if (timed) return timed;
  }
  const limit = details.message?.match(SIZE_LIMIT_PATTERN)?.[1];
  if (limit !== undefined) {
    const limited = translate(`codes.${code}_limit`, { limit: Number(limit) });
    if (limited) return limited;
  }
  return translate(`codes.${code}`);
}

export function translateApiError(details: ApiErrorDetails): string {
  return (
    translateCode(details) ??
    (details.status ? translate(`status.${details.status}`) : undefined) ??
    nonEmpty(details.message) ??
    translate("unknown") ??
    "Something went wrong. Please try again."
  );
}

export function networkErrorMessage(): string {
  return translate("network") ?? "Can't reach the server. Check your internet connection.";
}

export function timeoutErrorMessage(): string {
  return translate("timeout") ?? "The request took too long. Please try again.";
}

function envelope(error: unknown): ErrorEnvelope | undefined {
  if (!axios.isAxiosError(error)) return undefined;
  const data = (error as AxiosError<unknown>).response?.data;
  if (data && typeof data === "object") return data as ErrorEnvelope;
  return undefined;
}

function retryAfter(error: AxiosError): number | undefined {
  const raw = (error.response?.headers as Record<string, unknown> | undefined)?.["retry-after"];
  const value = Number(Array.isArray(raw) ? raw[0] : raw);
  return Number.isFinite(value) && value > 0 ? value : undefined;
}

export function getApiErrorMessage(error: unknown): string {
  if (error instanceof ApiError) return error.message;

  if (axios.isAxiosError(error)) {
    const ax = error as AxiosError<ErrorEnvelope>;

    if (!ax.response && (ax.code === "ERR_NETWORK" || ax.message === "Network Error")) {
      return networkErrorMessage();
    }
    if (ax.code === "ECONNABORTED" || ax.code === "ETIMEDOUT") {
      return timeoutErrorMessage();
    }

    const body = envelope(error);
    const serverMessage = nonEmpty(body?.message) ?? nonEmpty(body?.error);
    if (body?.code || ax.response?.status || serverMessage) {
      return translateApiError({
        code: body?.code,
        status: ax.response?.status,
        message: serverMessage,
        fields: body?.fields,
        retryAfterSeconds: retryAfter(ax),
      });
    }
  }

  if (error instanceof Error && error.message) {
    if (/^Request failed with status/i.test(error.message)) {
      return translateApiError({});
    }
    return error.message;
  }

  return translateApiError({});
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

export const isNotFoundError = (err: unknown): boolean =>
  err instanceof ApiError && (err.status === 404 || !!err.code?.endsWith("NOT_FOUND"));
