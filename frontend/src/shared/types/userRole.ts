export type UserRole = "student" | "teacher";

export const canCreateGroups = (role: UserRole | undefined): boolean => role === "teacher";
