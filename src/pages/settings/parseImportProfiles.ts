export type ImportProfile = { id: string; name: string; authentication_enabled: boolean };

export function parseImportProfiles(data: unknown): ImportProfile[] {
  if (
    typeof data !== "object" ||
    data === null ||
    !("schema_version" in data) ||
    (data.schema_version !== 1 && data.schema_version !== 2 && data.schema_version !== 3) ||
    !("profiles" in data) ||
    !Array.isArray(data.profiles)
  ) {
    throw new Error("Invalid configuration structure");
  }
  const profiles: ImportProfile[] = [];
  const ids = new Set<string>();
  for (const item of data.profiles as unknown[]) {
    if (
      typeof item !== "object" ||
      item === null ||
      !("id" in item) ||
      typeof item.id !== "string" ||
      !item.id ||
      ids.has(item.id) ||
      !("name" in item) ||
      typeof item.name !== "string" ||
      !item.name ||
      !("authentication_enabled" in item) ||
      typeof item.authentication_enabled !== "boolean"
    ) {
      throw new Error("Invalid profile structure");
    }
    ids.add(item.id);
    profiles.push({
      id: item.id,
      name: item.name,
      authentication_enabled: item.authentication_enabled,
    });
  }
  return profiles;
}
