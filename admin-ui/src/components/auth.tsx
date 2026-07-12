import { useState } from "react";
import { ArrowRight, Check, Database, HardDrive, Languages, Network, Server, ShieldCheck, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import logoImage from "../assets/logo.png";
import { api, type CurrentUser } from "../api";
import { rememberedLoginEmailStorageKey } from "../constants";
import type { ColorTheme, TranslatorContext } from "../types";
import type { Language } from "../i18n";
import { StatusDot, TextField } from "./common";
import { StorageLocationFields } from "./storage/storage-location-fields";

export function SetupOwnerForm({ t, onDone }: TranslatorContext & { onDone: (response: { accessToken: string; user: CurrentUser }) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setSubmitting(true);
    setError("");
    try {
      const response = await api.createOwner({ email, password, displayName: displayName || undefined });
      onDone(response);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form className="auth-form setup-form" onSubmit={submit}>
      <p>{t("setupHint")}</p>
      <TextField required label={t("email")} value={email} onChange={setEmail} />
      <TextField required label={t("password")} value={password} onChange={setPassword} type="password" />
      <TextField label={t("displayName")} value={displayName} onChange={setDisplayName} />
      {error && <div className="form-error">{error}</div>}
      <Button className="setup-primary-action" disabled={submitting} size="lg" type="submit">
        <span>{t("createAdministrator")}</span>
        <ArrowRight size={16} />
      </Button>
    </form>
  );
}

export function SetupLibraryForm({
  onDone,
  onSkip,
  t,
  token
}: TranslatorContext & {
  token: string;
  onDone: (libraryId: string) => void | Promise<void>;
  onSkip: () => void;
}) {
  const [name, setName] = useState("");
  const [workspaceKind, setWorkspaceKind] = useState("server_filesystem");
  const [workspaceCanonicalUri, setWorkspaceCanonicalUri] = useState("");
  const [error, setError] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!name.trim() || !workspaceCanonicalUri.trim()) {
      setError(t("formRequiredHint"));
      return;
    }

    setSubmitting(true);
    setError("");
    try {
      const library = await api.createLibrary(token, {
        displayName: name.trim(),
        defaultStorageRoot: {
          kind: workspaceKind,
          canonicalUri: workspaceCanonicalUri.trim()
        }
      });
      await onDone(library.id);
    } catch (nextError) {
      setError(nextError instanceof Error ? nextError.message : String(nextError));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form className="auth-form auth-library-form setup-form" onSubmit={submit}>
      <p>{t("firstLibraryHint")}</p>
      <TextField required label={t("name")} value={name} onChange={setName} />
      <StorageLocationFields
        kind={workspaceKind}
        location={workspaceCanonicalUri}
        t={t}
        onKindChange={setWorkspaceKind}
        onLocationChange={setWorkspaceCanonicalUri}
      />
      {error && <div className="form-error">{error}</div>}
      <div className="auth-form-actions">
        <Button disabled={submitting} type="button" variant="outline" onClick={onSkip}>{t("skipForNow")}</Button>
        <Button disabled={submitting} type="submit">
          <span>{t("createFirstLibrary")}</span>
          <ArrowRight size={16} />
        </Button>
      </div>
    </form>
  );
}

const setupSteps = ["setupStepWelcome", "setupStepOwner", "setupStepLibrary"] as const;

export function FirstRunSetup({
  colorTheme,
  language,
  setLanguage,
  t,
  token,
  onOwnerDone,
  onLibraryDone,
  onSkip
}: TranslatorContext & {
  colorTheme: ColorTheme;
  language: Language;
  setLanguage: (language: Language) => void;
  token: string | null;
  onOwnerDone: (response: { accessToken: string; user: CurrentUser }) => void;
  onLibraryDone: (libraryId: string) => void | Promise<void>;
  onSkip: () => void;
}) {
  const [step, setStep] = useState(0);
  const [setupToken, setSetupToken] = useState<string | null>(token);

  const ownerDone = (response: { accessToken: string; user: CurrentUser }) => {
    setSetupToken(response.accessToken);
    onOwnerDone(response);
    setStep(2);
  };

  return (
    <div className={`setup-shell ${colorTheme === "dark" ? "dark theme-dark" : "theme-light"}`}>
      <main className="setup-card">
        <div className="setup-viewport">
          <div className="setup-track" style={{ transform: `translate3d(-${step * 100}%, 0, 0)` }}>
            <section aria-hidden={step !== 0} className="setup-slide setup-welcome" inert={step !== 0}>
              <div className="setup-welcome-content">
                <img className="setup-hero-logo" alt="" src={logoImage} />
                <div className="setup-welcome-copy">
                  <h1>{t("setupWelcomeTitle")}</h1>
                  <p>{t("setupWelcomeHint")}</p>
                </div>
                <div className="setup-language-setting">
                  <div className="setup-language-options" role="radiogroup" aria-label={t("setupLanguageLabel")}>
                    <button
                      aria-checked={language === "zh"}
                      className={language === "zh" ? "is-active" : ""}
                      role="radio"
                      type="button"
                      onClick={() => setLanguage("zh")}
                    >
                      中文
                    </button>
                    <button
                      aria-checked={language === "en"}
                      className={language === "en" ? "is-active" : ""}
                      role="radio"
                      type="button"
                      onClick={() => setLanguage("en")}
                    >
                      English
                    </button>
                  </div>
                </div>
                <Button className="setup-start-button" size="lg" type="button" onClick={() => setStep(1)}>
                  <span>{t("startSetup")}</span>
                  <ArrowRight size={17} />
                </Button>
              </div>
            </section>

            <section aria-hidden={step !== 1} className="setup-slide" inert={step !== 1}>
              <div className="setup-slide-content setup-owner-content">
                <div className="setup-step-heading">
                  <h1>{t("setupOwnerTitle")}</h1>
                </div>
                <SetupOwnerForm t={t} onDone={ownerDone} />
              </div>
            </section>

            <section aria-hidden={step !== 2} className="setup-slide" inert={step !== 2}>
              <div className="setup-slide-content setup-library-content">
                <div className="setup-step-heading">
                  <h1>{t("firstLibrarySetup")}</h1>
                </div>
                {setupToken ? (
                  <SetupLibraryForm t={t} token={setupToken} onDone={onLibraryDone} onSkip={onSkip} />
                ) : (
                  <div className="auth-note">{t("loading")}</div>
                )}
              </div>
            </section>
          </div>
        </div>
        <nav aria-label={t("setupProgressLabel")} className="setup-progress">
          {setupSteps.map((label, index) => (
            <div aria-current={index === step ? "step" : undefined} className={`setup-progress-step${index === step ? " is-active" : ""}`} key={label}>
              <span aria-hidden="true" className="setup-progress-marker" />
              <span className="setup-progress-label">{t(label)}</span>
            </div>
          ))}
        </nav>
      </main>
    </div>
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
              <strong>{window.location.hostname || "127.0.0.1"}</strong>
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
