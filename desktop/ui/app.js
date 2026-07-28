import { detectLanguage, localizeBackendMessage, persistLanguage, translate } from "./i18n.js";

const invoke = window.__TAURI__.core.invoke;
const appWindow = window.__TAURI__.window.getCurrentWindow();
const elements = Object.fromEntries(Array.from(document.querySelectorAll("[id]")).map((node) => [node.id, node]));
const themeStorageKey = "starary_server_desktop_theme";
let language = detectLanguage();
let theme = detectTheme();
let status = null;
let busy = false;
let toastTimer;
let activePanel = "service";

const statusKeys = {
  database: { connected: "connected", stopped: "stopped", unknown: "unavailable" },
  storage: { writable: "writable", read_only: "readOnly", missing: "unavailable", unknown: "unavailable" }
};

function t(key, values) {
  return translate(language, key, values);
}

function detectTheme() {
  const stored = localStorage.getItem(themeStorageKey);
  if (stored === "system" || stored === "light" || stored === "dark") return stored;
  return "system";
}

function persistTheme(nextTheme) {
  localStorage.setItem(themeStorageKey, nextTheme);
}

function applyTheme() {
  document.documentElement.dataset.theme = theme;
  document.querySelectorAll("[data-theme-option]").forEach((button) => {
    const active = button.dataset.themeOption === theme;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  });
}

function applyLanguage() {
  document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-aria]").forEach((node) => {
    node.setAttribute("aria-label", t(node.dataset.i18nAria));
  });
  elements["language-select"].value = language;
  applyTheme();
  if (status) render(status);
  invoke("set_control_center_language", { language }).catch(() => {});
}

function showToast(message) {
  clearTimeout(toastTimer);
  elements.toast.textContent = message;
  elements.toast.classList.add("is-visible");
  toastTimer = setTimeout(() => elements.toast.classList.remove("is-visible"), 2200);
}

function showError(error) {
  showToast(localizeBackendMessage(error, language));
}

function setActivePanel(panelId) {
  activePanel = panelId;
  document.querySelectorAll("[data-panel-target]").forEach((button) => {
    const active = button.dataset.panelTarget === panelId;
    button.classList.toggle("is-active", active);
    if (active) {
      button.setAttribute("aria-current", "page");
    } else {
      button.removeAttribute("aria-current");
    }
  });
  document.querySelectorAll("[data-panel-id]").forEach((panel) => {
    panel.classList.toggle("is-active", panel.dataset.panelId === panelId);
  });
}

function render(next) {
  status = next;
  const running = next.state === "running" && next.managed;
  const conflict = next.state === "conflict";
  elements["service-toggle"].classList.toggle("is-on", running);
  elements["service-toggle"].setAttribute("aria-pressed", String(running));
  elements["service-toggle"].setAttribute("aria-label", t(running ? "stopService" : "startService"));
  elements["service-toggle"].title = t(running ? "stopService" : "startService");
  elements["service-toggle"].disabled = busy || conflict;
  elements["service-toggle-label"].textContent = t(running ? "stopService" : "startService");
  elements["service-toggle-hint"].textContent = t(running ? "stopServiceHint" : "startServiceHint");
  elements.message.textContent = localizeBackendMessage(next.message, language);
  elements["open-admin"].disabled = !running || busy;
  elements["copy-url"].disabled = !(next.lanUrl ?? next.localUrl);
  elements.port.disabled = running || busy;
  elements["save-port"].disabled = running || busy;
  elements.port.value = next.port;
  elements["launch-at-login"].checked = Boolean(next.launchAtLogin);
  elements["launch-at-login"].disabled = busy;
  elements["select-log-directory"].disabled = running || busy;
  if (document.activeElement !== elements["log-directory"]) {
    elements["log-directory"].value = next.logDirectory ?? "";
  }
  elements["service-url"].textContent = next.lanUrl ?? next.localUrl ?? "-";
  setHealth("http", running ? "running" : conflict ? "conflict" : "stopped", running, conflict);
  setHealth("database", statusKeys.database[next.databaseStatus] ?? "unavailable", next.databaseStatus === "connected", false);
  setHealth("storage", statusKeys.storage[next.storageStatus] ?? "unavailable", next.storageStatus === "writable", next.storageStatus === "read_only");
}

function setHealth(name, key, good, warn) {
  elements[`${name}-status`].textContent = t(key);
  elements[`${name}-dot`].className = `mini-dot${good ? " is-on" : warn ? " is-warn" : ""}`;
}

async function refresh(silent = true) {
  try {
    render(await invoke("get_service_status"));
  } catch (error) {
    if (!silent) showError(error);
  }
}

async function run(command, successKey, args) {
  if (busy) return;
  busy = true;
  if (status) render(status);
  try {
    render(await invoke(command, args));
    if (successKey) showToast(t(successKey));
  } catch (error) {
    showError(error);
    await refresh();
  } finally {
    busy = false;
    if (status) render(status);
  }
}

elements["language-select"].addEventListener("change", () => {
  language = elements["language-select"].value;
  persistLanguage(language);
  applyLanguage();
});
elements["service-toggle"].addEventListener("click", () => {
  const running = status?.state === "running";
  run(running ? "stop_service" : "start_service", running ? "serviceStoppedToast" : "serviceStartedToast");
});
elements["save-port"].addEventListener("click", () => run("change_service_port", "portSavedToast", { port: Number(elements.port.value) }));
elements["launch-at-login"].addEventListener("change", () => {
  run("set_launch_at_login", elements["launch-at-login"].checked ? "launchAtLoginEnabledToast" : "launchAtLoginDisabledToast", {
    enabled: elements["launch-at-login"].checked
  });
});
elements["select-log-directory"].addEventListener("click", async () => {
  if (busy) return;
  busy = true;
  if (status) render(status);
  try {
    const next = await invoke("select_log_directory");
    if (next) {
      render(next);
      showToast(t("logDirectorySavedToast"));
    }
  } catch (error) {
    showError(error);
    await refresh();
  } finally {
    busy = false;
    if (status) render(status);
  }
});
elements["open-admin"].addEventListener("click", () => invoke("open_admin").catch(showError));
elements["copy-url"].addEventListener("click", async () => {
  const url = status?.lanUrl ?? status?.localUrl;
  if (!url) return;
  try {
    await navigator.clipboard.writeText(url);
    showToast(t("addressCopiedToast"));
  } catch (error) {
    showError(error);
  }
});
elements["open-log"].addEventListener("click", () => invoke("open_log").catch(showError));
elements["check-updates"].addEventListener("click", () => showToast(t("updateCheckPlaceholderToast")));
elements["device-management"].addEventListener("click", () => showToast(t("deviceManagementPlanned")));
document.querySelectorAll("[data-panel-target]").forEach((button) => {
  button.addEventListener("click", () => setActivePanel(button.dataset.panelTarget));
});
document.querySelectorAll("[data-theme-option]").forEach((button) => {
  button.addEventListener("click", () => {
    theme = button.dataset.themeOption;
    persistTheme(theme);
    applyTheme();
  });
});
document.querySelectorAll("[data-window-action]").forEach((button) => {
  button.addEventListener("click", () => button.dataset.windowAction === "minimize" ? appWindow.minimize() : appWindow.close());
});

applyTheme();
applyLanguage();
setActivePanel(activePanel);
refresh(false);
setInterval(() => { if (!busy) refresh(); }, 2500);
