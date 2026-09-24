export function syncSystemTheme() {
  const preference = window.matchMedia("(prefers-color-scheme: dark)");
  const applyTheme = () => document.documentElement.classList.toggle("dark", preference.matches);

  applyTheme();
  preference.addEventListener("change", applyTheme);
  return () => preference.removeEventListener("change", applyTheme);
}
