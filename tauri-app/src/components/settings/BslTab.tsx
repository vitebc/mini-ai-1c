import React from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { FileCode, Download, RefreshCw, Cpu, CheckCircle, AlertTriangle, AlertCircle, Info, Terminal, ExternalLink } from 'lucide-react';
import { BslStatus, AppSettings, BslDiagnosticItem } from '../../types/settings';

const BSL_RELEASES_URL = 'https://github.com/1c-syntax/bsl-language-server/releases/latest';

interface BslTabProps {
    settings: AppSettings;
    setSettings: (settings: AppSettings) => void;
    bslStatus: BslStatus | null;
    refreshBslStatus: () => void;
    browseJar: () => void;
    handleDownloadBslLs: () => void;
    downloading: boolean;
    downloadProgress: number;
    downloadError?: string | null;
    downloadSuccess?: boolean;
    diagnosing: boolean;
    diagReport: BslDiagnosticItem[] | null;
    setDiagReport: (report: BslDiagnosticItem[] | null) => void;
    runDiagnostics: () => void;
    reconnectBsl: () => void;
    reconnecting: boolean;
}

export function BslTab({
    settings,
    setSettings,
    bslStatus,
    refreshBslStatus,
    browseJar,
    handleDownloadBslLs,
    downloading,
    downloadProgress,
    downloadError,
    downloadSuccess,
    diagnosing,
    diagReport,
    setDiagReport,
    runDiagnostics,
    reconnectBsl,
    reconnecting
}: BslTabProps) {
    const usesBundledRuntime = bslStatus?.runtime_info.includes('Встроенный') ?? false;
    const runtimeAvailable = usesBundledRuntime || (bslStatus?.java_info.includes('version') ?? false);

    return (
        <div className="p-4 sm:p-8 w-full h-full overflow-y-auto">
            <div className="max-w-2xl mx-auto space-y-6 sm:space-y-8">
                <section>
                    <h3 className="text-lg font-medium mb-4 flex items-center gap-2 text-zinc-100">
                        <FileCode className="w-5 h-5 text-blue-500" />
                        BSL Language Server
                    </h3>
                    <div className="bg-zinc-800/50 border border-zinc-700 rounded-xl p-5 space-y-4">
                        <div className="flex items-center gap-2 mb-4">
                            <input
                                type="checkbox"
                                checked={settings.bsl_server.enabled}
                                onChange={(e) => setSettings({
                                    ...settings,
                                    bsl_server: { ...settings.bsl_server, enabled: e.target.checked }
                                })}
                                className="rounded bg-zinc-700 border-zinc-600 text-blue-500 focus:ring-blue-500"
                            />
                            <span className="font-medium text-zinc-200">Enable BSL Language Server</span>
                        </div>

                        <div>
                            <label className="text-xs text-zinc-500 uppercase font-semibold mb-1 block">JAR Path</label>
                            <div className="flex flex-col sm:flex-row gap-2">
                                <input
                                    type="text"
                                    value={settings.bsl_server.jar_path}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        bsl_server: { ...settings.bsl_server, jar_path: e.target.value }
                                    })}
                                    className="flex-1 bg-[var(--input-bg)] border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:outline-none text-zinc-100"
                                />
                                <div className="flex gap-2">
                                    <button onClick={browseJar} className="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-lg text-sm text-zinc-200 transition-colors">Browse</button>
                                    <button
                                        onClick={handleDownloadBslLs}
                                        disabled={downloading}
                                        className="px-3 py-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 border border-green-700 rounded-lg text-sm text-white flex items-center gap-1 transition-colors"
                                    >
                                        <Download className="w-3 h-3" />
                                        {downloading ? 'Downloading...' : 'Download'}
                                    </button>
                                </div>
                            </div>
                            {downloading && (
                                <div className="mt-2 space-y-1">
                                    <div className="flex justify-between text-[10px] text-zinc-500 font-bold uppercase tracking-wider">
                                        <span className="flex items-center gap-1">
                                            <RefreshCw className="w-3 h-3 animate-spin" />
                                            Загрузка сервера...
                                        </span>
                                        <span>{downloadProgress}%</span>
                                    </div>
                                    <div className="w-full h-1.5 bg-zinc-800 border border-zinc-700 rounded-full overflow-hidden">
                                        <div
                                            className="h-full bg-gradient-to-r from-blue-600 to-blue-400 transition-all duration-300"
                                            style={{ width: `${downloadProgress}%` }}
                                        />
                                    </div>
                                </div>
                            )}

                            {/* Download error */}
                            {downloadError && (
                                <div className="mt-2 p-3 bg-red-500/10 border border-red-500/30 rounded-lg space-y-2">
                                    <div className="flex items-start gap-2 text-xs text-red-400">
                                        <AlertCircle className="w-3.5 h-3.5 mt-0.5 shrink-0" />
                                        <span className="break-all">{downloadError}</span>
                                    </div>
                                    <div className="text-xs text-zinc-400 flex items-center gap-1">
                                        Не удалось установить официальный Windows-пакет:
                                        <button
                                            onClick={() => openUrl(BSL_RELEASES_URL)}
                                            className="inline-flex items-center gap-1 text-blue-400 hover:text-blue-300 underline"
                                        >
                                            GitHub Releases <ExternalLink className="w-3 h-3" />
                                        </button>
                                    </div>
                                </div>
                            )}

                            {/* Download success */}
                            {downloadSuccess && !downloadError && (
                                <div className="mt-2 flex items-center gap-2 text-xs text-green-400">
                                    <CheckCircle className="w-3.5 h-3.5" />
                                    BSL Language Server успешно установлен
                                </div>
                            )}
                        </div>

                        <div>
                            <label className="text-xs text-zinc-500 uppercase font-semibold mb-1 block">Java Path</label>
                            <input
                                type="text"
                                value={settings.bsl_server.java_path}
                                onChange={(e) => setSettings({
                                    ...settings,
                                    bsl_server: { ...settings.bsl_server, java_path: e.target.value }
                                })}
                                className="w-full bg-[var(--input-bg)] border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:outline-none text-zinc-100"
                            />
                        </div>

                        <div className="border-t border-zinc-700 pt-4 mt-2">
                            <label className="text-xs text-zinc-500 uppercase font-semibold mb-1 block">Режим подключения</label>
                            <div className="flex items-center gap-3 mb-3">
                                <span className="text-xs text-zinc-500">Локальный (Java + JAR)</span>
                                <input
                                    type="checkbox"
                                    checked={!!settings.bsl_server.remote_url}
                                    onChange={(e) => setSettings({
                                        ...settings,
                                        bsl_server: {
                                            ...settings.bsl_server,
                                            remote_url: e.target.checked ? 'ws://' : ''
                                        }
                                    })}
                                    className="rounded bg-zinc-700 border-zinc-600 text-blue-500 focus:ring-blue-500"
                                />
                                <span className="text-xs text-zinc-500">Удалённый сервер</span>
                            </div>

                            {settings.bsl_server.remote_url ? (
                                <div>
                                    <label className="text-xs text-zinc-500 uppercase font-semibold mb-1 block">WebSocket URL сервера</label>
                                    <input
                                        type="text"
                                        value={settings.bsl_server.remote_url}
                                        onChange={(e) => setSettings({
                                            ...settings,
                                            bsl_server: { ...settings.bsl_server, remote_url: e.target.value }
                                        })}
                                        placeholder="ws://192.168.1.100:8025/lsp"
                                        className="w-full bg-[var(--input-bg)] border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:outline-none text-zinc-100 font-mono"
                                    />
                                </div>
                            ) : (
                                <>
                                    <div>
                                        <label className="text-xs text-zinc-500 uppercase font-semibold mb-1 block">WebSocket Port</label>
                                        <input
                                            type="number"
                                            value={settings.bsl_server.websocket_port}
                                            onChange={(e) => setSettings({
                                                ...settings,
                                                bsl_server: { ...settings.bsl_server, websocket_port: parseInt(e.target.value) || 8025 }
                                            })}
                                            className="w-32 bg-[var(--input-bg)] border border-zinc-700 rounded-lg px-3 py-2 text-sm focus:ring-2 focus:ring-blue-500 focus:outline-none text-zinc-100"
                                        />
                                    </div>
                                </>
                            )}
                        </div>
                    </div>
                </section>

                <section>
                    <h3 className="text-lg font-medium mb-4 flex items-center gap-2 text-zinc-100">
                        <RefreshCw className={`w-5 h-5 ${bslStatus?.connected ? 'text-green-400' : 'text-zinc-500'}`} />
                        Состояние системы
                    </h3>

                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-6">
                        <div className="bg-zinc-800/40 border border-zinc-700/50 rounded-xl p-4 flex flex-col items-center text-center">
                            <div className={`p-2 rounded-full mb-3 ${runtimeAvailable ? 'bg-green-500/10 text-green-400' : 'bg-red-500/10 text-red-400'}`}>
                                <Cpu className="w-5 h-5" />
                            </div>
                            <div className="text-xs text-zinc-500 font-medium uppercase mb-1">Runtime</div>
                            <div className="text-sm font-semibold truncate w-full text-zinc-200" title={bslStatus?.runtime_info}>
                                {runtimeAvailable ? (usesBundledRuntime ? 'Встроенный' : 'External Java') : 'Не найден'}
                            </div>
                        </div>

                        <div className="bg-zinc-800/40 border border-zinc-700/50 rounded-xl p-4 flex flex-col items-center text-center">
                            <div className={`p-2 rounded-full mb-3 ${bslStatus?.installed ? 'bg-green-500/10 text-green-400' : 'bg-red-500/10 text-red-400'}`}>
                                <FileCode className="w-5 h-5" />
                            </div>
                            <div className="text-xs text-zinc-500 font-medium uppercase mb-1">BSL Server</div>
                            <div className="text-sm font-semibold text-zinc-200" title={bslStatus?.server_path}>
                                {bslStatus?.installed ? (bslStatus.server_version || 'Legacy') : 'Отсутствует'}
                            </div>
                        </div>

                        <div className="bg-zinc-800/40 border border-zinc-700/50 rounded-xl p-4 flex flex-col items-center text-center">
                            <div className={`p-2 rounded-full mb-3 ${bslStatus?.connected ? 'bg-green-500/10 text-green-400' : 'bg-red-500/10 text-red-400'}`}>
                                <RefreshCw className={`w-5 h-5 ${bslStatus?.connected ? 'animate-spin-slow' : ''}`} />
                            </div>
                            <div className="text-xs text-zinc-500 font-medium uppercase mb-1">LSP Статус</div>
                            <div className="text-sm font-semibold text-zinc-200">
                                {bslStatus?.connected ? `Online :${bslStatus.active_port}` : 'Offline'}
                            </div>
                        </div>

                        <div className="bg-zinc-800/40 border border-zinc-700/50 rounded-xl p-4 flex flex-col items-center text-center">
                            <div className={`p-2 rounded-full mb-3 ${bslStatus?.mcp_available ? 'bg-green-500/10 text-green-400' : 'bg-zinc-700/50 text-zinc-500'}`}>
                                <Terminal className="w-5 h-5" />
                            </div>
                            <div className="text-xs text-zinc-500 font-medium uppercase mb-1">Official MCP</div>
                            <div className="text-sm font-semibold text-zinc-200">
                                {bslStatus?.mcp_available ? 'Доступен' : 'Недоступен'}
                            </div>
                        </div>
                    </div>

                    {/* Diagnose button */}
                    <div className="flex gap-3">
                        <button
                            onClick={reconnectBsl}
                            disabled={reconnecting}
                            className="flex items-center justify-center gap-2 px-4 py-2.5 bg-blue-600 hover:bg-blue-500 border border-blue-500 rounded-xl text-sm font-medium text-white transition-colors disabled:opacity-50"
                        >
                            <RefreshCw className={`w-4 h-4 ${reconnecting ? 'animate-spin' : ''}`} />
                            {reconnecting ? 'Подключение...' : 'Reconnect'}
                        </button>

                        <button
                            onClick={runDiagnostics}
                            disabled={diagnosing}
                            className="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-xl text-sm font-medium text-zinc-200 transition-all hover:scale-[1.02] active:scale-[0.98] disabled:opacity-50"
                        >
                            <Terminal className={`w-4 h-4 ${diagnosing ? 'animate-pulse' : ''}`} />
                            {diagnosing ? 'Выполняется диагностика...' : 'Запустить диагностику'}
                        </button>

                        <button
                            onClick={refreshBslStatus}
                            className="p-2.5 bg-zinc-800 hover:bg-zinc-700 border border-zinc-700 rounded-xl text-zinc-400 hover:text-zinc-200 transition-all"
                            title="Обновить статус"
                        >
                            <RefreshCw className="w-4 h-4" />
                        </button>
                    </div>

                    {/* Diagnostic report */}
                    {diagReport && (
                        <div className="mt-6 space-y-3 animate-in fade-in slide-in-from-top-4 duration-300">
                            <div className="flex items-center justify-between">
                                <h4 className="text-sm font-semibold text-zinc-400 uppercase tracking-wider">Результаты диагностики</h4>
                                <button onClick={() => setDiagReport(null)} className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors">Очистить</button>
                            </div>

                            <div className="space-y-3">
                                {diagReport.map((item, idx) => (
                                    <div
                                        key={idx}
                                        className={`p-4 rounded-xl border flex gap-4 ${item.status === 'ok' ? 'bg-green-500/5 border-green-500/20' :
                                            item.status === 'warn' ? 'bg-amber-500/5 border-amber-500/20' :
                                                'bg-red-500/5 border-red-500/20'
                                            }`}
                                    >
                                        <div className={`shrink-0 p-2 h-fit rounded-lg ${item.status === 'ok' ? 'bg-green-500/10 text-green-400' :
                                            item.status === 'warn' ? 'bg-amber-500/10 text-amber-400' :
                                                'bg-red-500/10 text-red-400'
                                            }`}>
                                            {item.status === 'ok' ? <CheckCircle className="w-5 h-5" /> :
                                                item.status === 'warn' ? <AlertTriangle className="w-5 h-5" /> :
                                                    <AlertCircle className="w-5 h-5" />}
                                        </div>
                                        <div className="flex-1 space-y-1">
                                            <div className="font-semibold text-sm text-zinc-100">{item.title}</div>
                                            <div className="text-sm text-zinc-400 leading-relaxed">{item.message}</div>
                                            {item.suggestion && (
                                                <div className="mt-2 text-xs flex items-start gap-2 text-zinc-300 bg-white/5 p-2 rounded-lg">
                                                    <Info className="w-4 h-4 shrink-0 mt-0.5 text-blue-400" />
                                                    <div>{item.suggestion}</div>
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}
                </section>
            </div>
        </div>
    );
}
