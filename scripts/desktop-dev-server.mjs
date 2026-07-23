import { spawn } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pidFile = join(root, "target", "build-dev", "desktop", "control-center.pid");
const viteEntry = join(root, "admin-ui", "node_modules", "vite", "bin", "vite.js");
const frontendRoot = join(root, "target", "build-dev", "frontend", "ui");
const startupDeadline = Date.now() + 5 * 60 * 1000;

rmSync(pidFile, { force: true });

const vite = spawn(
  process.execPath,
  [viteEntry, frontendRoot, "--host", "127.0.0.1", "--port", "14310", "--strictPort"],
  {
    cwd: root,
    stdio: "inherit",
    windowsHide: true,
  },
);

let desktopStarted = false;
let stopping = false;

function isProcessRunning(processId) {
  if (!Number.isInteger(processId) || processId <= 0) {
    return false;
  }
  try {
    process.kill(processId, 0);
    return true;
  } catch {
    return false;
  }
}

function readDesktopProcessId() {
  if (!existsSync(pidFile)) {
    return null;
  }
  const processId = Number.parseInt(readFileSync(pidFile, "utf8").trim(), 10);
  return Number.isInteger(processId) ? processId : null;
}

function stop(exitCode = 0) {
  if (stopping) {
    return;
  }
  stopping = true;
  clearInterval(monitor);
  rmSync(pidFile, { force: true });
  if (!vite.killed) {
    vite.kill();
  }
  const forceTimer = setTimeout(() => {
    if (vite.exitCode === null) {
      vite.kill("SIGKILL");
    }
  }, 2000);
  forceTimer.unref();
  if (vite.exitCode !== null) {
    process.exit(exitCode);
    return;
  }
  vite.once("exit", () => process.exit(exitCode));
}

const monitor = setInterval(() => {
  const desktopProcessId = readDesktopProcessId();
  if (desktopProcessId && isProcessRunning(desktopProcessId)) {
    desktopStarted = true;
    return;
  }
  if (desktopStarted || Date.now() >= startupDeadline) {
    stop(desktopStarted ? 0 : 1);
  }
}, 500);

vite.once("exit", (code) => {
  if (!stopping) {
    stop(code ?? 1);
  }
});

process.once("SIGINT", () => stop(0));
process.once("SIGTERM", () => stop(0));
process.once("SIGHUP", () => stop(0));
