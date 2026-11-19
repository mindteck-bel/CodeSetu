/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import {
  THEME_STORAGE_KEY,
  applyTheme,
  cycleTheme,
  readThemePreference,
  resolveTheme,
  themeLabel,
} from "./theme";

afterEach(() => {
  localStorage.removeItem(THEME_STORAGE_KEY);
  delete document.documentElement.dataset.theme;
  delete document.documentElement.dataset.themePreference;
});

describe("theme helpers", () => {
  it("defaults preference to system", () => {
    expect(readThemePreference()).toBe("system");
  });

  it("reads saved preference", () => {
    localStorage.setItem(THEME_STORAGE_KEY, "light");
    expect(readThemePreference()).toBe("light");
  });

  it("resolves explicit preferences", () => {
    expect(resolveTheme("dark")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
  });

  it("cycles dark → light → system → dark", () => {
    expect(cycleTheme("dark")).toBe("light");
    expect(cycleTheme("light")).toBe("system");
    expect(cycleTheme("system")).toBe("dark");
  });

  it("applies and persists theme", () => {
    const resolved = applyTheme("light");
    expect(resolved).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.dataset.themePreference).toBe("light");
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe("light");
    expect(themeLabel("light")).toBe("Light");
  });
});
