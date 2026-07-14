const languageStorageKey = "madlibrary_server_desktop_language";

export const translations = {
  zh: {
    seats: "\u5e2d\u4f4d",
    brandServer: "\u670d\u52a1\u7aef",
    console: "\u63a7\u5236\u53f0",
    settings: "\u8bbe\u7f6e",
    generalSettings: "\u901a\u7528",
    systemSettings: "\u7cfb\u7edf",
    language: "\u8bed\u8a00",
    languagePreference: "\u754c\u9762\u8bed\u8a00",
    languagePreferenceHint: "\u9009\u62e9\u684c\u9762\u7aef\u663e\u793a\u8bed\u8a00",
    languageChinese: "\u7b80\u4f53\u4e2d\u6587",
    languageEnglish: "English",
    theme: "\u4e3b\u9898",
    themePreference: "\u5916\u89c2\u4e3b\u9898",
    themePreferenceHint: "\u8ddf\u968f\u7cfb\u7edf\u6216\u624b\u52a8\u9009\u62e9",
    themeSystem: "\u8ddf\u968f\u7cfb\u7edf",
    themeLight: "\u6d45\u8272",
    themeDark: "\u6df1\u8272",
    servicePanelTitle: "\u670d\u52a1",
    servicePanelDescription: "\u542f\u52a8\u3001\u505c\u6b62\u5e76\u76d1\u63a7\u56e2\u961f\u670d\u52a1",
    consolePanelTitle: "\u63a7\u5236\u53f0",
    consolePanelDescription: "\u6253\u5f00\u540e\u53f0\u63a7\u5236\u53f0\u8fdb\u884c\u7ba1\u7406",
    logsActionDescription: "\u6253\u5f00\u670d\u52a1\u65e5\u5fd7",
    dataActionDescription: "\u6253\u5f00\u672c\u5730\u670d\u52a1\u6570\u636e",
    devicePanelDescription: "\u7ba1\u7406\u5df2\u8fde\u63a5\u5e2d\u4f4d\u548c\u53ef\u4fe1\u8bbe\u5907",
    settingsPanelDescription: "\u8c03\u6574\u670d\u52a1\u684c\u9762\u7aef\u504f\u597d",
    deviceManagement: "\u8bbe\u5907\u7ba1\u7406",
    deviceManagementPlanned: "\u8bbe\u5907\u7ba1\u7406\u5c06\u5728\u540e\u7eed\u7248\u672c\u5f00\u653e",
    minimize: "最小化",
    closeToTray: "关闭到系统托盘",
    controlCenter: "服务控制中心",
    teamService: "Mad Library 团队服务",
    switchToEnglish: "切换至 English",
    startService: "启动服务",
    startServiceHint: "启动本机服务",
    stopService: "停止服务",
    stopServiceHint: "停止当前运行的服务",
    checking: "检查中",
    checkingInstance: "正在识别本机服务实例",
    serviceRunning: "服务运行中",
    serviceConflict: "服务识别冲突",
    serviceStopped: "服务已停止",
    portStatus: "端口 {port}",
    unmanagedPort: "当前端口不是此控制中心管理的实例",
    portReleased: "端口已释放",
    openAdmin: "打开后台管理",
    openAdminHint: "管理资源库、储存、用户、权限等",
    service: "服务",
    database: "数据库",
    storage: "储存",
    running: "运行中",
    conflict: "冲突",
    stopped: "已停止",
    connected: "已连接",
    unavailable: "不可用",
    writable: "可写",
    readOnly: "只读",
    serviceAddress: "服务地址",
    copy: "复制",
    servicePort: "服务端口",
    servicePortHint: "\u5148\u7ec8\u6b62\u670d\u52a1\u624d\u53ef\u4fee\u6539",
    logDirectory: "日志目录",
    logDirectoryHint: "先停止服务才可修改",
    save: "保存",
    openDirectory: "打开",
    selectDirectory: "选择文件夹",
    openLogFile: "打开日志文件",
    restart: "重启",
    localMaintenance: "本机维护",
    logsAndData: "日志与服务数据",
    logs: "日志",
    dataDirectory: "数据目录",
    dataDirectoryHint: "打开本地服务数据",
    serviceStartedToast: "服务已启动",
    serviceStoppedToast: "服务已停止",
    serviceRestartedToast: "服务已重新启动",
    portSavedToast: "端口已保存",
    logDirectorySavedToast: "日志目录已保存",
    addressCopiedToast: "地址已复制"
  },
  en: {
    seats: "Seats",
    brandServer: "Server",
    console: "Console",
    settings: "Settings",
    generalSettings: "General",
    systemSettings: "System",
    language: "Language",
    languagePreference: "Interface language",
    languagePreferenceHint: "Choose the desktop display language",
    languageChinese: "简体中文",
    languageEnglish: "English",
    theme: "Theme",
    themePreference: "Appearance theme",
    themePreferenceHint: "Follow system or choose manually",
    themeSystem: "System",
    themeLight: "Light",
    themeDark: "Dark",
    servicePanelTitle: "Service",
    servicePanelDescription: "Start, stop, and monitor the team service",
    consolePanelTitle: "Console",
    consolePanelDescription: "Open the admin console to manage the server",
    logsActionDescription: "Open service logs",
    dataActionDescription: "Open local service data",
    devicePanelDescription: "Manage connected seats and trusted devices",
    settingsPanelDescription: "Adjust local shell preferences",
    deviceManagement: "Device management",
    deviceManagementPlanned: "Device management will be available in a future version",
    minimize: "Minimize",
    closeToTray: "Close to system tray",
    controlCenter: "Service Control Center",
    teamService: "Mad Library Team Server",
    switchToChinese: "切换至中文",
    startService: "Start service",
    startServiceHint: "Start the local service",
    stopService: "Stop service",
    stopServiceHint: "Stop the running service",
    checking: "Checking",
    checkingInstance: "Identifying the local service instance",
    serviceRunning: "Service running",
    serviceConflict: "Service conflict",
    serviceStopped: "Service stopped",
    portStatus: "Port {port}",
    unmanagedPort: "The process on this port is not managed by this control center",
    portReleased: "Port available",
    openAdmin: "Open Admin",
    openAdminHint: "Manage libraries, storage, users, permissions, and more",
    service: "Service",
    database: "Database",
    storage: "Storage",
    running: "Running",
    conflict: "Conflict",
    stopped: "Stopped",
    connected: "Connected",
    unavailable: "Unavailable",
    writable: "Writable",
    readOnly: "Read only",
    serviceAddress: "Service address",
    copy: "Copy",
    servicePort: "Service port",
    servicePortHint: "Stop the service before changing it",
    logDirectory: "Log directory",
    logDirectoryHint: "Stop the service before changing it",
    save: "Save",
    openDirectory: "Open",
    selectDirectory: "Choose folder",
    openLogFile: "Open log file",
    restart: "Restart",
    localMaintenance: "Local tools",
    logsAndData: "Logs and service data",
    logs: "Logs",
    dataDirectory: "Data folder",
    dataDirectoryHint: "Open local service data",
    serviceStartedToast: "Service started",
    serviceStoppedToast: "Service stopped",
    serviceRestartedToast: "Service restarted",
    portSavedToast: "Port saved",
    logDirectorySavedToast: "Log directory saved",
    addressCopiedToast: "Address copied"
  }
};

const englishBackendMessages = new Map([
  ["该端口已被其他程序占用，控制中心不会接管或停止它。", "This port is in use by another program. The control center will not manage or stop it."],
  ["检测到另一套或非受管的 Mad Library 服务，控制中心不会接管它。", "Another or unmanaged Mad Library service was detected. The control center will not take it over."],
  ["服务端口冲突", "Service port conflict."],
  ["当前端口上的进程不属于此控制中心，已拒绝停止。", "The process on this port is not managed by this control center and was not stopped."],
  ["服务拒绝了停止请求。", "The service rejected the stop request."],
  ["服务未能在预期时间内停止，请检查服务日志。", "The service did not stop in time. Check the service log."],
  ["端口必须在 1024 到 65535 之间。", "The port must be between 1024 and 65535."],
  ["请先停止服务，再修改端口。", "Stop the service before changing the port."],
  ["请先停止服务，再修改日志目录。", "Stop the service before changing the log directory."],
  ["日志目录不能为空。", "The log directory cannot be empty."],
  ["服务尚未运行。", "The service is not running."],
  ["服务操作正在进行中。", "A service operation is already in progress."]
]);

export function detectLanguage() {
  const stored = localStorage.getItem(languageStorageKey);
  if (stored === "zh" || stored === "en") return stored;
  return "zh";
}

export function persistLanguage(language) {
  localStorage.setItem(languageStorageKey, language);
}

export function translate(language, key, values = {}) {
  const template = translations[language]?.[key] ?? translations.zh[key] ?? key;
  return Object.entries(values).reduce(
    (result, [name, value]) => result.replaceAll(`{${name}}`, String(value)),
    template
  );
}

export function localizeBackendMessage(message, language) {
  const text = String(message ?? "");
  if (!text || language === "zh") return text;
  if (englishBackendMessages.has(text)) return englishBackendMessages.get(text);
  if (text.startsWith("端口 ") && text.endsWith(" 已被占用。")) {
    return `Port ${text.slice(3, -6)} is already in use.`;
  }
  if (text.startsWith("服务启动失败，请检查日志：")) {
    return `Failed to start the service. Check the log: ${text.slice(12)}`;
  }
  if (text.startsWith("服务启动超时，请检查日志：")) {
    return `Service startup timed out. Check the log: ${text.slice(12)}`;
  }
  if (text.startsWith("服务程序不存在：")) {
    return `The service executable was not found: ${text.slice(8)}`;
  }
  return text;
}
