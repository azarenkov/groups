import axios, { AxiosError } from "axios";

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

export function getApiErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const ax = error as AxiosError<any>;

    if (ax.code === "ERR_NETWORK" || ax.message === "Network Error") {
      return "Can't reach the server. Check your internet connection.";
    }
    if (ax.code === "ECONNABORTED") {
      return "The request took too long. Try again.";
    }

    const data = ax.response?.data;
    if (data && typeof data === "object") {
      if (typeof data.message === "string" && data.message.trim().length > 0) {
        return data.message;
      }
      if (typeof data.error === "string" && data.error.trim().length > 0) {
        return data.error;
      }
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

export function toApiError(error: unknown): Error {
  if (error instanceof Error && !axios.isAxiosError(error)) {
    return error;
  }
  return new Error(getApiErrorMessage(error));
}
