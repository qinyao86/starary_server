import { RefreshCw, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { api, type ServerTask } from "../api";
import { PageFrame, Panel, StatusDot, UserAvatar } from "../components/common";
import type { PageContext } from "../types";
import { canManageServerRole, formatDateTime } from "../utils/format";

const activeStatuses = new Set(["running", "paused", "pausing", "cancelling"]);

export function TasksPage({ t, token, currentUser, setMessage }: PageContext) {
  const [tasks, setTasks] = useState<ServerTask[]>([]);
  const [busy, setBusy] = useState(false);
  const [deletingTaskId, setDeletingTaskId] = useState<string | null>(null);
  const canDeleteTasks = canManageServerRole(currentUser?.role ?? "");

  const load = useCallback(async () => {
    setBusy(true);
    try {
      const response = await api.listTasks(token);
      setTasks(response.items);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [setMessage, token]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      void load();
    }, 5000);
    return () => window.clearInterval(intervalId);
  }, [load]);

  const deleteTask = async (task: ServerTask) => {
    if (!canDeleteTasks || deletingTaskId) return;
    if (!window.confirm(`${t("taskDeleteTitle")}\n\n${t("taskDeleteHint")}`)) {
      return;
    }
    setDeletingTaskId(task.id);
    try {
      const updated = await api.deleteTask(token, task.id);
      if (updated.deletedAt) {
        setTasks((current) => current.filter((item) => item.id !== task.id));
        setMessage(t("taskDeleted"));
      } else {
        setTasks((current) => current.map((item) => (item.id === updated.id ? updated : item)));
        setMessage(t("taskDeleteRequested"));
      }
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setDeletingTaskId(null);
    }
  };

  const summary = useMemo(() => {
    const active = tasks.filter((task) => activeStatuses.has(task.status)).length;
    return `${active}/${tasks.length}`;
  }, [tasks]);

  return (
    <PageFrame
      title={t("tasks")}
      description={t("tasksPageHint")}
      action={
        <Button disabled={busy} size="sm" type="button" variant="outline" onClick={() => void load()}>
          <RefreshCw size={15} />
          {t("refresh")}
        </Button>
      }
    >
      <Panel
        title={`${t("taskList")} (${summary})`}
        icon={RefreshCw}
        className="span-12 tasks-panel"
        action={<span className="tasks-auto-refresh">{t("taskAutoRefresh")}</span>}
      >
        <div className="table-wrap tasks-table-wrap">
          <table className="data-table tasks-table">
            <thead>
              <tr>
                <th>{t("name")}</th>
                <th>{t("taskProgress")}</th>
                <th>{t("taskOwner")}</th>
                <th>{t("taskLibrary")}</th>
                <th>{t("taskHeartbeat")}</th>
                <th>{t("action")}</th>
              </tr>
            </thead>
            <tbody>
              {tasks.length === 0 ? (
                <tr>
                  <td colSpan={6}>{t("taskEmpty")}</td>
                </tr>
              ) : tasks.map((task) => (
                <tr key={task.id}>
                  <td>
                    <div className="task-name-cell">
                      <strong>{task.title}</strong>
                      <span>{task.jobType} · {task.id}</span>
                      {task.message && <em>{task.message}</em>}
                    </div>
                  </td>
                  <td>
                    <div className="task-progress-cell">
                      <StatusDot label={taskStatusLabel(t, task)} tone={taskStatusTone(task)} />
                      <div className="task-progress-bar" aria-hidden="true">
                        <span style={{ width: `${Math.max(0, Math.min(100, task.progress))}%` }} />
                      </div>
                      <span>{task.processedCount}/{task.totalCount} · {task.progress}%</span>
                    </div>
                  </td>
                  <td>
                    <div className="task-user-cell">
                      <UserAvatar avatarKey={task.userAvatarKey} label={task.userDisplayName ?? task.userEmail ?? "-"} size="md" userId={task.userId ?? undefined} />
                      <div>
                        <strong>{task.userDisplayName ?? "-"}</strong>
                        <span>{task.userEmail ?? "-"}</span>
                      </div>
                    </div>
                  </td>
                  <td>{task.libraryName ?? task.libraryId ?? "-"}</td>
                  <td>{formatDateTime(task.lastHeartbeatAt)}</td>
                  <td>
                    <Button
                      aria-label={t("delete")}
                      disabled={!canDeleteTasks || deletingTaskId === task.id}
                      size="icon"
                      title={t("delete")}
                      type="button"
                      variant="ghost"
                      onClick={() => void deleteTask(task)}
                    >
                      <Trash2 size={15} />
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Panel>
    </PageFrame>
  );
}

function taskStatusLabel(t: PageContext["t"], task: ServerTask) {
  if (task.deleteRequestedAt && !task.deletedAt) return t("taskStatusDeleteRequested");
  switch (task.status) {
    case "running":
      return t("taskStatusRunning");
    case "paused":
      return t("taskStatusPaused");
    case "pausing":
      return t("taskStatusPausing");
    case "completed":
      return t("taskStatusCompleted");
    case "completed_with_errors":
      return t("taskStatusCompletedWithErrors");
    case "failed":
      return t("taskStatusFailed");
    case "cancelling":
      return t("taskStatusCancelling");
    case "cancelled":
      return t("taskStatusCancelled");
    default:
      return task.status;
  }
}

function taskStatusTone(task: ServerTask): "good" | "warn" | "muted" {
  if (task.deleteRequestedAt && !task.deletedAt) return "warn";
  if (task.status === "completed") return "good";
  if (task.status === "failed") return "warn";
  if (task.status === "cancelled" || task.status === "paused") return "muted";
  return "warn";
}
