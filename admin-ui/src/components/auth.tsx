import { useState } from "react";
import { Check, Database, HardDrive, Languages, Network, Server, ShieldCheck, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import logoImage from "../assets/logo.png";
import { api, type CurrentUser } from "../api";
import { rememberedLoginEmailStorageKey } from "../constants";
import type { ColorTheme, TranslatorContext } from "../types";
import type { Language } from "../i18n";
import { StatusDot, TextField } from "./common";

export function SetupOwnerForm({ t, onDone }: TranslatorContext & { onDone: (response: { accessToken: string; user: CurrentUser }) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState("");

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      const response = await api.createOwner({ email, password, displayName: displayName || undefined });
      onDone(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  };

  return (
    <form className="auth-form" onSubmit={submit}>
      <p>{t("setupHint")}</p>
      <TextField label={t("email")} value={email} onChange={setEmail} />
      <TextField label={t("password")} value={password} onChange={setPassword} type="password" />
      <TextField label={t("displayName")} value={displayName} onChange={setDisplayName} />
      {error && <div className="form-error">{error}</div>}
      <button className="primary-button" type="submit">{t("createOwner")}</button>
    </form>
  );
}

export function LoginForm({ t, onDone }: TranslatorContext & { onDone: (response: { accessToken: string; user: CurrentUser }) => void }) {
  const [email, setEmail] = useState(() => localStorage.getItem(rememberedLoginEmailStorageKey) ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    try {
      const normalizedEmail = email.trim();
      const response = await api.login({ email: normalizedEmail, password });
      localStorage.setItem(rememberedLoginEmailStorageKey, normalizedEmail);
      onDone(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    }
  };

  return (
    <form className="auth-form" onSubmit={submit}>
      <p>{t("loginHint")}</p>
      <TextField label={t("email")} value={email} onChange={setEmail} />
      <TextField label={t("password")} value={password} onChange={setPassword} type="password" />
      {error && <div className="form-error">{error}</div>}
      <button className="primary-button" type="submit">{t("login")}</button>
    </form>
  );
}

export function AuthShell({
  t,
  language,
  setLanguage,
  colorTheme,
  title,
  children
}: TranslatorContext & {
  language: Language;
  setLanguage: (language: Language) => void;
  colorTheme: ColorTheme;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className={`auth-shell ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <div className="auth-frame">
        <section className="auth-visual">
          <div className="auth-product">
            <div className="auth-logo-lockup">
              <div className="brand-mark"><img alt="" src={logoImage} /></div>
              <div>
                <div className="auth-product-name">{t("appName")}</div>
                <div className="auth-product-mode">{t("managedRuntime")}</div>
              </div>
            </div>
            <StatusDot label={t("online")} tone="good" />
          </div>

          <div className="auth-visual-copy">
            <div className="auth-kicker">{t("commandCenter")}</div>
            <h2>{t("controlPlane")}</h2>
            <p>{t("secureInit")}</p>
          </div>

          <div className="auth-system-grid">
            <div className="auth-system-card">
              <Server size={18} />
              <span>{t("localNode")}</span>
              <strong>127.0.0.1</strong>
            </div>
            <div className="auth-system-card">
              <Database size={18} />
              <span>{t("databaseOnline")}</span>
              <strong>PostgreSQL</strong>
            </div>
            <div className="auth-system-card">
              <Network size={18} />
              <span>{t("apiGateway")}</span>
              <strong>/api/v1</strong>
            </div>
            <div className="auth-system-card">
              <HardDrive size={18} />
              <span>{t("storageReady")}</span>
              <strong>{t("local")}</strong>
            </div>
          </div>

          <div className="auth-boot-panel">
            <div className="auth-boot-title">{t("bootSequence")}</div>
            <div className="auth-boot-row"><Check size={14} /> {t("databaseOnline")}</div>
            <div className="auth-boot-row"><Check size={14} /> {t("storageReady")}</div>
            <div className="auth-boot-row is-pending"><ShieldCheck size={14} /> {t("ownerProfile")}</div>
          </div>
        </section>

        <section className="auth-card">
          <div className="brand auth-brand">
            <div className="auth-brand-lockup">
              <div className="brand-mark"><img alt="" src={logoImage} /></div>
              <div>
                <div className="brand-title">{t("appName")}</div>
                <div className="brand-subtitle">{t("admin")}</div>
              </div>
            </div>
            <button className="select-button auth-language-button" type="button" onClick={() => setLanguage(language === "zh" ? "en" : "zh")}>
              <Languages size={16} />
              <span>{language === "zh" ? "\u4e2d\u6587" : "English"}</span>
            </button>
          </div>
          <div className="auth-header">
            <h1>{title}</h1>
          </div>
          {children ?? <div className="auth-note">{t("loading")}</div>}
        </section>
      </div>
    </div>
  );
}
