import { useCallback, useEffect, useState } from "react";
import { useApp } from "../../app/AppContext";
import {
  api,
  type BackupInfo,
  type BackupKind,
  type BackupSettings,
  type IntegrityReport,
} from "../../shared/api";
import { errMsg } from "../../shared/text";
import { Button, ConfirmDialog, ErrorBanner, Field, inputCls } from "../../shared/ui";

const KIND_LABEL: Record<BackupKind, string> = {
  manual: "Manual",
  auto: "Automático",
  "pre-restore": "Antes de restaurar",
};

const fmtDate = (iso: string) => {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("pt-BR");
};

const fmtSize = (b: number) =>
  b < 1024 ? `${b} B` : b < 1048576 ? `${(b / 1024).toFixed(1)} KB` : `${(b / 1048576).toFixed(1)} MB`;

function Report({ report }: { report: IntegrityReport }) {
  return (
    <ul className="mt-2 space-y-0.5 text-xs">
      {report.checks.map((c) => (
        <li
          key={c.name}
          className={c.ok ? "text-emerald-600 dark:text-emerald-400" : "text-rose-600 dark:text-rose-400"}
        >
          {c.ok ? "✓" : "✗"} {c.name} — {c.detail}
        </li>
      ))}
    </ul>
  );
}

export function BackupView() {
  const { project, enterProject, invalidate } = useApp();
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [settings, setSettings] = useState<BackupSettings | null>(null);
  const [label, setLabel] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [reports, setReports] = useState<Record<string, IntegrityReport>>({});
  const [live, setLive] = useState<IntegrityReport | null>(null);
  const [restoring, setRestoring] = useState<BackupInfo | null>(null);
  const [deleting, setDeleting] = useState<BackupInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const [list, s] = await Promise.all([api.listBackups(), api.getBackupSettings()]);
      setBackups(list);
      setSettings(s);
    } catch (e) {
      setError(errMsg(e));
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  async function run(key: string, fn: () => Promise<void>) {
    setBusy(key);
    setError(null);
    setNotice(null);
    try {
      await fn();
    } catch (e) {
      setError(errMsg(e));
    } finally {
      setBusy(null);
    }
  }

  const createNow = () =>
    run("create", async () => {
      await api.createBackup(label.trim() || undefined);
      setLabel("");
      await load();
      setNotice("Backup criado e verificado.");
    });

  const verify = (id: string) =>
    run(id, async () => {
      const r = await api.verifyBackup(id);
      setReports((prev) => ({ ...prev, [id]: r }));
    });

  const checkLive = () =>
    run("live", async () => {
      setLive(await api.checkProjectIntegrity());
    });

  const saveSettings = () =>
    run("settings", async () => {
      if (!settings) return;
      setSettings(await api.setBackupSettings(settings));
      setNotice("Configurações salvas.");
    });

  async function doRestore() {
    if (!restoring) return;
    const target = restoring;
    setRestoring(null);
    await run("restore", async () => {
      const info = await api.restoreBackup(target.id);
      await enterProject(info); // recarrega universos e tipos do banco restaurado
      invalidate();
      await load();
      setNotice(
        "Backup restaurado. O estado anterior foi guardado como backup “Antes de restaurar”.",
      );
    });
  }

  async function doDelete() {
    if (!deleting) return;
    const target = deleting;
    setDeleting(null);
    await run(target.id, async () => {
      await api.deleteBackup(target.id);
      await load();
    });
  }

  return (
    <div className="h-full overflow-y-auto p-6">
      <div className="mx-auto max-w-4xl space-y-8">
        <header>
          <h2 className="text-lg font-semibold">Backup e integridade</h2>
          <p className="text-sm text-zinc-500">
            Pasta dos backups:{" "}
            <code className="rounded bg-zinc-200 px-1 text-xs dark:bg-zinc-800">
              {project?.path}/backups
            </code>
            . Copie-a de tempos em tempos para outro disco ou nuvem: backups no mesmo disco não
            protegem contra falha do disco.
          </p>
        </header>

        <ErrorBanner message={error} onClose={() => setError(null)} />
        {notice && (
          <div className="rounded-md border border-emerald-300 bg-emerald-50 px-3 py-2 text-sm text-emerald-800 dark:border-emerald-900 dark:bg-emerald-950 dark:text-emerald-200">
            {notice}
          </div>
        )}

        {/* Criar backup + verificar banco atual */}
        <section className="space-y-3 rounded-md border border-zinc-200 p-4 dark:border-zinc-800">
          <h3 className="text-sm font-semibold">Criar backup agora</h3>
          <div className="flex gap-2">
            <input
              className={inputCls}
              placeholder="Descrição (opcional), ex.: antes de reorganizar os tipos"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              maxLength={80}
            />
            <Button onClick={createNow} disabled={busy !== null}>
              {busy === "create" ? "Criando…" : "Criar backup"}
            </Button>
          </div>

          <div className="flex items-center gap-3 border-t border-zinc-200 pt-3 dark:border-zinc-800">
            <Button variant="secondary" onClick={checkLive} disabled={busy !== null}>
              {busy === "live" ? "Verificando…" : "Verificar integridade do projeto"}
            </Button>
            {live && (
              <span
                className={`text-sm ${live.ok ? "text-emerald-600" : "text-rose-600"}`}
              >
                {live.ok ? "Tudo certo." : "Problemas encontrados."}
              </span>
            )}
          </div>
          {live && <Report report={live} />}
        </section>

        {/* Backup automático */}
        {settings && (
          <section className="space-y-3 rounded-md border border-zinc-200 p-4 dark:border-zinc-800">
            <h3 className="text-sm font-semibold">Backup automático</h3>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={settings.autoEnabled}
                onChange={(e) => setSettings({ ...settings, autoEnabled: e.target.checked })}
              />
              Criar backups automaticamente enquanto o UWB estiver aberto
            </label>
            <div className="grid max-w-md gap-3 sm:grid-cols-2">
              <Field label="Intervalo (horas)" hint="De 1 a 720.">
                <input
                  type="number"
                  min={1}
                  max={720}
                  className={inputCls}
                  value={settings.intervalHours}
                  disabled={!settings.autoEnabled}
                  onChange={(e) =>
                    setSettings({ ...settings, intervalHours: Number(e.target.value) })
                  }
                />
              </Field>
              <Field label="Manter os últimos" hint="Só vale para os automáticos.">
                <input
                  type="number"
                  min={1}
                  max={100}
                  className={inputCls}
                  value={settings.keepAuto}
                  disabled={!settings.autoEnabled}
                  onChange={(e) => setSettings({ ...settings, keepAuto: Number(e.target.value) })}
                />
              </Field>
            </div>
            <Button variant="secondary" onClick={saveSettings} disabled={busy !== null}>
              Salvar configurações
            </Button>
          </section>
        )}

        {/* Lista */}
        <section className="space-y-3">
          <h3 className="text-sm font-semibold">Backups ({backups.length})</h3>
          {backups.length === 0 && <p className="text-sm text-zinc-500">Nenhum backup ainda.</p>}
          {backups.map((b) => {
            const r = reports[b.id];
            return (
              <div
                key={b.id}
                className="rounded-md border border-zinc-200 p-3 dark:border-zinc-800"
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="font-medium">{fmtDate(b.createdAt)}</span>
                      <span className="rounded-full bg-zinc-200 px-2 py-0.5 text-[10px] text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                        {KIND_LABEL[b.kind] ?? b.kind}
                      </span>
                    </div>
                    {b.label && <div className="text-sm text-zinc-600 dark:text-zinc-400">{b.label}</div>}
                    <div className="text-xs text-zinc-500">
                      {fmtSize(b.sizeBytes)} · {b.counts.entities} entidades ·{" "}
                      {b.counts.entityTypes} tipos · {b.counts.relations} relações ·{" "}
                      {b.counts.universes} universo(s)
                    </div>
                  </div>
                  <div className="flex gap-1">
                    <Button variant="ghost" disabled={busy !== null} onClick={() => verify(b.id)}>
                      {busy === b.id ? "…" : "Verificar"}
                    </Button>
                    <Button variant="ghost" disabled={busy !== null} onClick={() => setRestoring(b)}>
                      Restaurar
                    </Button>
                    <Button variant="ghost" disabled={busy !== null} onClick={() => setDeleting(b)}>
                      Excluir
                    </Button>
                  </div>
                </div>
                {r && (
                  <div className="mt-2 border-t border-zinc-200 pt-2 dark:border-zinc-800">
                    <div
                      className={`text-sm font-medium ${r.ok ? "text-emerald-600" : "text-rose-600"}`}
                    >
                      {r.ok ? "Backup íntegro." : "Backup com problemas — não use para restaurar."}
                    </div>
                    <Report report={r} />
                  </div>
                )}
              </div>
            );
          })}
        </section>
      </div>

      <ConfirmDialog
        open={restoring !== null}
        danger
        title="Restaurar este backup?"
        confirmLabel="Restaurar"
        message={
          <>
            <p>
              O projeto voltará ao estado de <b>{restoring ? fmtDate(restoring.createdAt) : ""}</b>.
              Tudo o que foi feito depois será substituído.
            </p>
            <p>
              Antes de restaurar, o estado atual é guardado automaticamente como um backup “Antes
              de restaurar”. Alterações não salvas em edição serão perdidas.
            </p>
          </>
        }
        onConfirm={doRestore}
        onCancel={() => setRestoring(null)}
      />

      <ConfirmDialog
        open={deleting !== null}
        danger
        title="Excluir este backup?"
        confirmLabel="Excluir backup"
        message={<p>O backup será apagado do disco. Essa ação não pode ser desfeita.</p>}
        onConfirm={doDelete}
        onCancel={() => setDeleting(null)}
      />
    </div>
  );
}