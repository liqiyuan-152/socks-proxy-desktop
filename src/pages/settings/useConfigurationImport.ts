import { ipc } from "@/lib/ipc";
import { useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/backend";
import { parseImportProfiles, type ImportProfile } from "./parseImportProfiles";

type Credential = { username: string; password: string };

export function useConfigurationImport(onImported: () => Promise<void>) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<ImportProfile[]>([]);
  const [credentials, setCredentials] = useState<Record<string, Credential>>({});
  const json = useRef("");
  const epoch = useRef(0);
  const submitting = useRef(false);
  const active = useRef(false);

  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
      epoch.current += 1;
      json.current = "";
    };
  }, []);

  function reset() {
    epoch.current += 1;
    json.current = "";
    setOpen(false);
    setProfiles([]);
    setCredentials({});
    setError(null);
  }

  function close() {
    if (!submitting.current) reset();
  }

  async function read(file: File) {
    if (submitting.current) return;
    reset();
    const version = epoch.current;
    const current = () => active.current && epoch.current === version;
    let text: string;
    try {
      text = await file.text();
    } catch {
      if (current()) setError("无法读取配置文件，请重新选择文件。");
      return;
    }
    if (!current()) return;
    let data: unknown;
    try {
      data = JSON.parse(text);
    } catch {
      setError("配置文件不是有效的 JSON。");
      return;
    }
    try {
      const preview = parseImportProfiles(data);
      json.current = text;
      setProfiles(preview);
      setOpen(true);
    } catch {
      setError("配置文件结构无效，请选择本应用导出的配置。");
    }
  }

  async function submit() {
    if (!active.current || !open || submitting.current) return;
    const updates: [string, Credential & { action: "replace" }][] = [];
    for (const profile of profiles.filter((item) => item.authentication_enabled)) {
      const credential = credentials[profile.id];
      if (!credential?.username.trim() || !credential.password) {
        setError(`请为「${profile.name}」重新输入认证用户名和密码。`);
        return;
      }
      updates.push([profile.id, { action: "replace", ...credential }]);
    }
    submitting.current = true;
    setBusy(true);
    setError(null);
    const version = epoch.current;
    try {
      await ipc("import_configuration", {
        json: json.current,
        updates: Object.fromEntries(updates),
      });
      if (!active.current || epoch.current !== version) return;
      reset();
      await onImported();
    } catch (reason) {
      if (active.current) setError(errorMessage(reason));
    } finally {
      submitting.current = false;
      if (active.current) setBusy(false);
    }
  }

  function updateCredential(id: string, field: keyof Credential, value: string) {
    setCredentials((current) => ({
      ...current,
      [id]: { ...(current[id] ?? { username: "", password: "" }), [field]: value },
    }));
  }

  return { open, busy, error, profiles, credentials, read, submit, close, updateCredential };
}
