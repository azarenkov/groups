import "@testing-library/jest-dom";
import { TextDecoder, TextEncoder } from "util";

if (typeof globalThis.TextEncoder === "undefined") {
  Object.assign(globalThis, { TextEncoder, TextDecoder });
}

beforeEach(() => {
  (globalThis as { __riftQueryClient?: { clear: () => void } }).__riftQueryClient?.clear();
});
