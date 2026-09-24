export type ImportProfile = { id: string; name: string; authentication_enabled: boolean };

export function parseImportProfiles(data: unknown): ImportProfile[] {
  if (
    typeof data !== "object" ||
    data === null ||
    !("profiles" in data) ||
    !Array.isArray(data.profiles)
  )
    return [];
  return data.profiles.filter(
    (item: unknown): item is ImportProfile =>
      typeof item === "object" &&
      item !== null &&
      "id" in item &&
      typeof item.id === "string" &&
      "name" in item &&
      typeof item.name === "string" &&
      "authentication_enabled" in item &&
      typeof item.authentication_enabled === "boolean",
  );
}
