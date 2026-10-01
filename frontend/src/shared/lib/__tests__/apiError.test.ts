import { AxiosError, AxiosHeaders, InternalAxiosRequestConfig } from "axios";

import i18n from "../../../i18n";
import { ApiError, getApiErrorMessage, toApiError, translateApiError } from "../apiError";

const config = { headers: new AxiosHeaders() } as InternalAxiosRequestConfig;

function httpError(
  status: number,
  data: unknown,
  headers: Record<string, string> = {},
): AxiosError {
  return new AxiosError(`Request failed with status code ${status}`, "ERR_BAD_REQUEST", config, {}, {
    status,
    statusText: "",
    headers,
    config,
    data,
  });
}

describe("apiError translations", () => {
  afterEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("translates a known code into the active language", async () => {
    const error = httpError(401, {
      code: "INVALID_CREDENTIALS",
      message: "Invalid credentials",
      status: 401,
    });

    await i18n.changeLanguage("en");
    expect(toApiError(error).message).toBe("Wrong email or password.");

    await i18n.changeLanguage("ru");
    const translated = toApiError(error);
    expect(translated).toBeInstanceOf(ApiError);
    expect(translated.message).toBe("Неверная почта или пароль.");
    expect((translated as ApiError).code).toBe("INVALID_CREDENTIALS");
  });

  it("falls back to the status text for an unknown code", async () => {
    const error = httpError(403, { code: "SOMETHING_NEW", message: "Raw server text" });

    expect(getApiErrorMessage(error)).toBe("You don't have access to this.");

    await i18n.changeLanguage("ru");
    expect(getApiErrorMessage(error)).toBe("У вас нет доступа к этому.");
  });

  it("falls back to the server message when neither code nor status is known", async () => {
    await i18n.changeLanguage("ru");
    const error = httpError(418, { code: "SOMETHING_NEW", message: "Raw server text" });

    expect(getApiErrorMessage(error)).toBe("Raw server text");
    expect(translateApiError({ code: "NOPE", status: 499, message: "From server" })).toBe(
      "From server",
    );
  });

  it("interpolates the wait time for resend limits", async () => {
    await i18n.changeLanguage("ru");
    const error = httpError(429, {
      code: "VERIFICATION_RESEND_TOO_SOON",
      message: "Too many codes requested. Try again in 42 seconds.",
    });

    expect(getApiErrorMessage(error)).toBe(
      "Слишком много запросов кода. Попробуйте снова через 42 с.",
    );
  });

  it("keeps the size limit from the server", async () => {
    await i18n.changeLanguage("ru");
    const error = httpError(413, {
      code: "AVATAR_TOO_LARGE",
      message: "Image is too large (max 8 MB).",
    });

    expect(getApiErrorMessage(error)).toBe("Изображение слишком большое: максимум 8 МБ.");
  });

  it("uses field codes for validation failures", () => {
    const error = httpError(400, {
      code: "VALIDATION_FAILED",
      message: "Validation failed",
      fields: [{ field: "email", code: "email", message: "invalid" }],
    });

    expect(getApiErrorMessage(error)).toBe("Enter a valid email address.");
  });

  it("translates network failures and timeouts", async () => {
    await i18n.changeLanguage("ru");
    const network = new AxiosError("Network Error", "ERR_NETWORK", config);
    const timeout = new AxiosError("timeout of 10000ms exceeded", "ECONNABORTED", config);

    expect(getApiErrorMessage(network)).toBe(
      "Нет связи с сервером. Проверьте подключение к интернету.",
    );
    expect(getApiErrorMessage(timeout)).toBe(
      "Запрос выполнялся слишком долго. Попробуйте ещё раз.",
    );
  });
});
