export type Locale = "en" | "ru";

export const SUPPORTED_LOCALES: Locale[] = ["en", "ru"];

export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  ru: "Русский",
};

export type Dict = {
  auth: {
    welcome: string;
    welcomeBack: string;
    signInSubtitle: string;
    createSubtitle: string;
    name: string;
    namePlaceholder: string;
    email: string;
    emailPlaceholder: string;
    password: string;
    passwordPlaceholder: string;
    signIn: string;
    createAccount: string;
    needAccount: string;
    haveAccount: string;
    submittingSignIn: string;
    submittingRegister: string;
    invalidCredentials: string;
    genericError: string;
    verifyTitle: string;
    verifyDescription: string;
    verifyCodePlaceholder: string;
    verifyButton: string;
    verifyResend: string;
    verifyResentNotice: string;
    languageLabel: string;
  };
  settings: {
    language: string;
    languageDescription: string;
    saving: string;
    saved: string;
    failed: string;
  };
  common: {
    cancel: string;
    save: string;
    close: string;
  };
};

export const STRINGS: Record<Locale, Dict> = {
  en: {
    auth: {
      welcome: "Welcome to Rift",
      welcomeBack: "Welcome back",
      signInSubtitle: "Sign in to continue",
      createSubtitle: "Create your account to get started",
      name: "Username",
      namePlaceholder: "your_username",
      email: "Email",
      emailPlaceholder: "you@example.com",
      password: "Password",
      passwordPlaceholder: "••••••••",
      signIn: "Sign in",
      createAccount: "Create account",
      needAccount: "Don't have an account? Sign up",
      haveAccount: "Already have an account? Sign in",
      submittingSignIn: "Signing in…",
      submittingRegister: "Creating account…",
      invalidCredentials: "Invalid email or password",
      genericError: "Something went wrong. Please try again.",
      verifyTitle: "Verify your email",
      verifyDescription: "We sent a 6-digit code to your email. Enter it below.",
      verifyCodePlaceholder: "Verification code",
      verifyButton: "Verify",
      verifyResend: "Resend code",
      verifyResentNotice: "Code re-sent.",
      languageLabel: "Language",
    },
    settings: {
      language: "Language",
      languageDescription: "Choose the language used for the app and emails.",
      saving: "Saving…",
      saved: "Saved",
      failed: "Failed to save",
    },
    common: {
      cancel: "Cancel",
      save: "Save",
      close: "Close",
    },
  },
  ru: {
    auth: {
      welcome: "Добро пожаловать в Rift",
      welcomeBack: "С возвращением",
      signInSubtitle: "Войдите, чтобы продолжить",
      createSubtitle: "Создайте аккаунт, чтобы начать",
      name: "Имя пользователя",
      namePlaceholder: "ваш_логин",
      email: "Почта",
      emailPlaceholder: "you@example.com",
      password: "Пароль",
      passwordPlaceholder: "••••••••",
      signIn: "Войти",
      createAccount: "Создать аккаунт",
      needAccount: "Нет аккаунта? Зарегистрируйтесь",
      haveAccount: "Уже есть аккаунт? Войдите",
      submittingSignIn: "Входим…",
      submittingRegister: "Создаём аккаунт…",
      invalidCredentials: "Неверный email или пароль",
      genericError: "Что-то пошло не так. Попробуйте ещё раз.",
      verifyTitle: "Подтвердите почту",
      verifyDescription: "Мы отправили 6-значный код на вашу почту. Введите его ниже.",
      verifyCodePlaceholder: "Код подтверждения",
      verifyButton: "Подтвердить",
      verifyResend: "Отправить ещё раз",
      verifyResentNotice: "Код отправлен повторно.",
      languageLabel: "Язык",
    },
    settings: {
      language: "Язык",
      languageDescription: "Выберите язык интерфейса и писем.",
      saving: "Сохраняем…",
      saved: "Сохранено",
      failed: "Не удалось сохранить",
    },
    common: {
      cancel: "Отмена",
      save: "Сохранить",
      close: "Закрыть",
    },
  },
};
