import { ipc } from "@/lib/ipc";
import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import {
  errorMessage,
  type BackendError,
  type ProfileCredential,
  type ProxyProfile,
} from "@/lib/backend";
import type { CredentialUpdate } from "@/lib/generated/credentials";
import type { ProxyDraft } from "./ProxyAuthenticationFields";
import { createProxyDraft } from "./proxyDraft";
import { parseProxyLink } from "./parseProxyLink";

type EditorOptions = {
  busy: boolean;
  setBusy: Dispatch<SetStateAction<boolean>>;
  setError: Dispatch<SetStateAction<string | null>>;
  refresh: () => Promise<void>;
  clearLatency: (id: string) => void;
};

export function useProxyEditor({ busy, setBusy, setError, refresh, clearLatency }: EditorOptions) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingProxy, setEditingProxy] = useState<ProxyProfile | null>(null);
  const [draft, setDraft] = useState<ProxyDraft>(() => createProxyDraft());
  const [proxyLink, setProxyLink] = useState("");
  const [linkError, setLinkError] = useState<string | null>(null);
  const [credentialLoading, setCredentialLoading] = useState(false);
  const [credentialError, setCredentialError] = useState<string | null>(null);
  const [originalCredential, setOriginalCredential] = useState<ProfileCredential | null>(null);
  const credentialRequest = useRef(0);
  const editorGeneration = useRef(0);
  const saving = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    const credentials = credentialRequest;
    const editor = editorGeneration;
    mounted.current = true;
    return () => {
      mounted.current = false;
      credentials.current++;
      editor.current++;
    };
  }, []);

  async function loadCredential(id: string) {
    const request = ++credentialRequest.current;
    setCredentialLoading(true);
    setCredentialError(null);
    try {
      const credential = await ipc("get_profile_credential", { id });
      if (request !== credentialRequest.current) return;
      setOriginalCredential(credential);
      setDraft((current) => ({ ...current, ...credential }));
    } catch (reason) {
      if (request === credentialRequest.current) setCredentialError(errorMessage(reason));
    } finally {
      if (request === credentialRequest.current) setCredentialLoading(false);
    }
  }

  async function saveProfile() {
    if (credentialLoading || credentialError || busy || saving.current) return;
    saving.current = true;
    const generation = editorGeneration.current;
    setBusy(true);
    setError(null);
    try {
      const credential: CredentialUpdate = !draft.authentication
        ? { action: "delete" }
        : originalCredential &&
            draft.username === originalCredential.username &&
            draft.password === originalCredential.password
          ? { action: "preserve" }
          : { action: "replace", username: draft.username, password: draft.password };
      await ipc("save_profile", {
        input: {
          id: editingProxy?.id ?? null,
          name: draft.name,
          protocol: draft.protocol,
          host: draft.server,
          port: Number(draft.port),
          authentication_enabled: draft.authentication,
          enabled: editingProxy?.enabled ?? true,
          credential,
        },
      });
      if (editingProxy) clearLatency(editingProxy.id);
      if (!mounted.current) return;
      if (generation === editorGeneration.current) closeDialog();
      await refresh();
    } catch (reason) {
      if (!mounted.current || generation !== editorGeneration.current) return;
      const typed = reason as Partial<BackendError>;
      setError(
        typed.fields?.map((field) => `${field.field}: ${field.message}`).join("；") ||
          errorMessage(reason),
      );
    } finally {
      saving.current = false;
      if (mounted.current) setBusy(false);
    }
  }

  const openDialog = (proxy?: ProxyProfile) => {
    editorGeneration.current++;
    credentialRequest.current++;
    setEditingProxy(proxy ?? null);
    setDraft(createProxyDraft(proxy));
    setOriginalCredential(null);
    setCredentialError(null);
    setCredentialLoading(false);
    setProxyLink("");
    setLinkError(null);
    setDialogOpen(true);
    if (proxy?.authentication_enabled) void loadCredential(proxy.id);
  };

  const closeDialog = () => {
    editorGeneration.current++;
    credentialRequest.current++;
    setDialogOpen(false);
    setEditingProxy(null);
    setOriginalCredential(null);
    setDraft(createProxyDraft());
    setCredentialError(null);
    setCredentialLoading(false);
  };

  const applyProxyLink = () => {
    try {
      const parsed = parseProxyLink(proxyLink);
      credentialRequest.current++;
      setOriginalCredential(null);
      setCredentialLoading(false);
      setCredentialError(null);
      setDraft(parsed);
      setProxyLink("");
      setLinkError(null);
    } catch (reason) {
      setLinkError(reason instanceof Error ? reason.message : "代理链接格式无效");
    }
  };

  return {
    dialogOpen,
    editingProxy,
    draft,
    setDraft,
    proxyLink,
    setProxyLink,
    linkError,
    setLinkError,
    credentialLoading,
    credentialError,
    loadCredential,
    openDialog,
    closeDialog,
    applyProxyLink,
    saveProfile,
  };
}
