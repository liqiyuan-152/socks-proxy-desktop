// Generated explicit credential IPC boundary. Do not edit or log values.
import type { ProxyProtocol } from "./ipc";
export type CredentialUpdate =
  | { action: "preserve" }
  | { action: "replace"; username: string; password: string }
  | { action: "delete" };
export type ProfileInput = {
  id: string | null;
  name: string;
  protocol: ProxyProtocol;
  host: string;
  port: number;
  authentication_enabled: boolean;
  enabled: boolean;
  credential: CredentialUpdate | null;
};
export type ProfileCredentialView = { username: string; password: string };
