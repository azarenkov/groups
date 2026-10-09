import React from "react";

export const ArrowLeft: React.FC<{ size?: number }> = ({ size = 14 }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.8}
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <line x1="19" y1="12" x2="5" y2="12" />
    <polyline points="12 19 5 12 12 5" />
  </svg>
);

interface OtpInputProps {
  value: string;
  onChange: (v: string) => void;
  autoFocus?: boolean;
}

export const OtpInput: React.FC<OtpInputProps> = ({ value, onChange, autoFocus }) => {
  const refs = React.useRef<(HTMLInputElement | null)[]>([]);
  const digits = (value || "").padEnd(6, " ").slice(0, 6).split("");

  const setAt = (i: number, ch: string) => {
    const next = digits.slice();
    next[i] = ch;
    onChange(next.join("").replace(/\s+$/, ""));
  };

  return (
    <div className="auth-otp">
      {digits.map((d, i) => (
        <input
          key={i}
          ref={(el) => {
            refs.current[i] = el;
          }}
          autoFocus={autoFocus && i === 0}
          inputMode="numeric"
          maxLength={1}
          className={d.trim() ? "filled" : ""}
          value={d.trim()}
          onChange={(e) => {
            const v = e.target.value.replace(/\D/g, "").slice(-1);
            setAt(i, v || " ");
            if (v && i < 5) refs.current[i + 1]?.focus();
          }}
          onKeyDown={(e) => {
            if (e.key === "Backspace" && !digits[i].trim() && i > 0) {
              refs.current[i - 1]?.focus();
              setAt(i - 1, " ");
              e.preventDefault();
            } else if (e.key === "ArrowLeft" && i > 0) refs.current[i - 1]?.focus();
            else if (e.key === "ArrowRight" && i < 5) refs.current[i + 1]?.focus();
          }}
          onPaste={(e) => {
            const pasted = e.clipboardData.getData("text").replace(/\D/g, "").slice(0, 6);
            if (!pasted) return;
            e.preventDefault();
            onChange(pasted);
            refs.current[Math.min(pasted.length, 5)]?.focus();
          }}
        />
      ))}
    </div>
  );
};

export type StrengthLevel = "veryWeak" | "weak" | "okay" | "strong" | "excellent";

export interface PasswordStrength {
  score: 0 | 1 | 2 | 3 | 4;
  level: StrengthLevel | null;
}

const LEVELS: StrengthLevel[] = ["veryWeak", "weak", "okay", "strong", "excellent"];

export const passwordStrength = (pw: string): PasswordStrength => {
  if (!pw) return { score: 0, level: null };
  let s = 0;
  if (pw.length >= 8) s++;
  if (pw.length >= 12) s++;
  if (/[A-Z]/.test(pw) && /[a-z]/.test(pw)) s++;
  if (/[0-9]/.test(pw) && /[^A-Za-z0-9]/.test(pw)) s++;
  return { score: s as PasswordStrength["score"], level: LEVELS[s] };
};

export const FormError: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div
    className="error-text"
    role="alert"
    style={{
      fontSize: 12,
      color: "var(--danger)",
      background: "var(--danger-tint)",
      padding: "8px 12px",
      borderRadius: 8,
      marginTop: 10,
      textAlign: "center",
    }}
  >
    {children}
  </div>
);
