import { useState, useEffect, useRef, useCallback } from 'react';
import { Search, ChevronDown, Check, Link, Unlink, MonitorOff, AlertTriangle, Play, FolderOpen, Server } from 'lucide-react';
import { useSettings } from '../../contexts/SettingsContext';
import { useConfigurator } from '../../contexts/ConfiguratorContext';
import {
  BUILTIN_1C_SEARCH_ID,
  normalizeSearchProfiles,
  buildSearchEnv,
} from '../../utils/searchProfiles';
import { launchConfigurator, get1cInfobases, get1cPlatformPath } from '../../api/mcp';

const BINDINGS_KEY = 'mcp_search_profile_bindings';

function loadBindings(): Record<string, Record<number, string>> {
  try {
    const raw = localStorage.getItem(BINDINGS_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveBindings(bindings: Record<string, Record<number, string>>): void {
  localStorage.setItem(BINDINGS_KEY, JSON.stringify(bindings));
}

interface Infobase {
  name: string;
  connection: string;
  type: 'file' | 'server';
  id: string | null;
  folder: string | null;
}

export function SearchProfileBar() {
  const { settings, updateSettings } = useSettings();
  const { selectedPid } = useConfigurator();
  const [isOpen, setIsOpen] = useState(false);
  const [showDatabases, setShowDatabases] = useState(false);
  const [isBound, setIsBound] = useState(false);
  const [infobases, setInfobases] = useState<Infobase[]>([]);
  const [platformPath, setPlatformPath] = useState<string>('');
  const [launching, setLaunching] = useState<string | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const searchServer = settings?.mcp_servers.find(
    s => s.id === BUILTIN_1C_SEARCH_ID && s.enabled,
  );
  const { profiles, activeId } = searchServer
    ? normalizeSearchProfiles(searchServer)
    : { profiles: [], activeId: '' };
  const activeProfile = profiles.find(p => p.id === activeId) || profiles[0];

  const jvvEnabled = settings?.mcp_servers.some(
    s => s.id === 'builtin-jvv-1c' && s.enabled,
  );

  // Fetch infobases and platform path on mount
  useEffect(() => {
    if (!jvvEnabled) return;
    let cancelled = false;
    (async () => {
      try {
        const [bases, platform] = await Promise.all([
          get1cInfobases().catch(() => []),
          get1cPlatformPath().catch(() => ''),
        ]);
        if (!cancelled) {
          setInfobases(bases);
          setPlatformPath(platform);
        }
      } catch { /* ignore */ }
    })();
    return () => { cancelled = true; };
  }, [jvvEnabled]);

  // Save binding: currentPid → profileId
  const saveBinding = useCallback((pid: number, profileId: string) => {
    const bindings = loadBindings();
    if (!bindings[BUILTIN_1C_SEARCH_ID]) bindings[BUILTIN_1C_SEARCH_ID] = {};
    bindings[BUILTIN_1C_SEARCH_ID][pid] = profileId;
    saveBindings(bindings);
    setIsBound(true);
  }, []);

  // Remove binding for currentPid
  const removeBinding = useCallback(() => {
    if (!selectedPid) return;
    const bindings = loadBindings();
    if (bindings[BUILTIN_1C_SEARCH_ID]) {
      delete bindings[BUILTIN_1C_SEARCH_ID][selectedPid];
      saveBindings(bindings);
    }
    setIsBound(false);
  }, [selectedPid]);

  // When selectedPid changes, check if there's a bound profile and auto-switch
  useEffect(() => {
    if (!searchServer || profiles.length === 0) {
      setIsBound(false);
      return;
    }

    // Auto-select if only one profile exists and no active profile
    if (profiles.length === 1 && activeId !== profiles[0].id) {
      const newEnv = buildSearchEnv(searchServer, profiles, profiles[0].id);
      updateSettings({
        ...settings!,
        mcp_servers: settings!.mcp_servers.map(s =>
          s.id === BUILTIN_1C_SEARCH_ID ? { ...s, env: newEnv } : s,
        ),
      });
      setIsBound(false);
      return;
    }

    if (!selectedPid) {
      setIsBound(false);
      return;
    }

    const bindings = loadBindings();
    const boundId = bindings[BUILTIN_1C_SEARCH_ID]?.[selectedPid];

    if (boundId && profiles.some(p => p.id === boundId) && boundId !== activeId) {
      // Auto-switch to bound profile
      const newEnv = buildSearchEnv(searchServer, profiles, boundId);
      updateSettings({
        ...settings!,
        mcp_servers: settings!.mcp_servers.map(s =>
          s.id === BUILTIN_1C_SEARCH_ID ? { ...s, env: newEnv } : s,
        ),
      });
      setIsBound(true);
    } else {
      setIsBound(!!boundId && profiles.some(p => p.id === boundId));
    }
  }, [selectedPid, searchServer, profiles, activeId, settings, updateSettings]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
        setShowDatabases(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Hide completely if MCP search is disabled
  if (!searchServer || profiles.length === 0) return null;

  // Profile mismatch: configurator is open, profile is selected, but no binding exists
  const isMismatch = !!selectedPid && !isBound && activeProfile != null;

  const handleSelect = (profileId: string) => {
    if (!settings) return;
    const newEnv = buildSearchEnv(searchServer, profiles, profileId);
    updateSettings({
      ...settings,
      mcp_servers: settings.mcp_servers.map(s =>
        s.id === BUILTIN_1C_SEARCH_ID ? { ...s, env: newEnv } : s,
      ),
    });
    // Save binding if we have a PID
    if (selectedPid) {
      saveBinding(selectedPid, profileId);
    }
    setIsOpen(false);
  };

  const handleLaunch = async (base: Infobase) => {
    if (!platformPath || !settings) return;
    setLaunching(base.name);
    try {
      // Bind the profile to this database before launching
      if (activeProfile) {
        updateSettings({
          ...settings,
          mcp_servers: settings.mcp_servers.map(s => {
            if (s.id !== BUILTIN_1C_SEARCH_ID) return s;
            const env = s.env || {};
            const profilesJson = env['ONEC_CONFIG_PROFILES_JSON'] || '[]';
            try {
              const profilesArr = JSON.parse(profilesJson);
              const updated = profilesArr.map((p: any) =>
                p.id === activeProfile.id
                  ? { ...p, bound_infobase: { name: base.name, connection: base.connection, type: base.type } }
                  : p
              );
              return {
                ...s,
                env: { ...env, 'ONEC_CONFIG_PROFILES_JSON': JSON.stringify(updated) },
              };
            } catch { return s; }
          }),
        });
      }
      // Read login/password from jvv-1c server config
      const jvvEnv = settings.mcp_servers.find(s => s.id === 'builtin-jvv-1c')?.env || {};
      const login = jvvEnv['ONEC_LOGIN'] || '';
      const password = jvvEnv['ONEC_PASSWORD'] || '';
      await launchConfigurator(platformPath, base.connection, base.type === 'server', login, password);
      setShowDatabases(false);
    } catch (e) {
      console.error('Launch failed:', e);
    } finally {
      setLaunching(null);
    }
  };

  return (
    <div className="max-w-4xl mx-auto w-full px-1 mb-2 relative" ref={dropdownRef}>
      <div className="flex items-center gap-1.5">
        {/* ── Кнопка баз 1С ── */}
        {jvvEnabled && (
          <div className="relative">
            <button
              onClick={() => { setShowDatabases(!showDatabases); setIsOpen(false); }}
              className="flex items-center gap-1 px-2 py-1 rounded-md text-[11px] transition-all text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50"
              title="Базы 1С на этом компьютере"
            >
              <Play className="w-3 h-3" />
              <span className="hidden min-[500px]:inline">Базы</span>
              {infobases.length > 0 && (
                <span className="text-[9px] px-1 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
                  {infobases.length}
                </span>
              )}
              <ChevronDown className={`w-2.5 h-2.5 transition-transform ${showDatabases ? 'rotate-180' : ''}`} />
            </button>

            {showDatabases && (
              <div className="absolute bottom-full left-0 mb-1 w-80 bg-zinc-800 border border-zinc-700 rounded-lg shadow-2xl z-50 py-1 animate-in slide-in-from-bottom-2 duration-200">
                <div className="px-3 py-1.5 border-b border-zinc-700 mb-1">
                  <span className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider">
                    Базы 1С на этом компьютере
                  </span>
                </div>
                <div className="max-h-[220px] overflow-y-auto custom-scrollbar">
                  {infobases.length === 0 ? (
                    <div className="px-3 py-3 text-[11px] text-zinc-500 italic">
                      {platformPath ? 'Базы 1С не найдены в ibases.v8i.' : 'Платформа и базы не найдены. Включите MCP «1С:Платформа и базы».'}
                    </div>
                  ) : (
                    infobases.map((base, idx) => (
                      <div
                        key={idx}
                        className="flex items-center justify-between gap-2 px-3 py-2 hover:bg-zinc-700/50 transition-colors group"
                      >
                        <div className="flex items-center gap-2 min-w-0 flex-1">
                          {base.type === 'file'
                            ? <FolderOpen className="w-3 h-3 text-zinc-500 shrink-0" />
                            : <Server className="w-3 h-3 text-blue-400 shrink-0" />}
                          <div className="min-w-0">
                            <div className="text-[12px] text-zinc-200 font-medium truncate">{base.name}</div>
                            <div className="text-[9px] text-zinc-500 truncate">{base.connection}</div>
                          </div>
                        </div>
                        <button
                          onClick={() => handleLaunch(base)}
                          disabled={!platformPath || launching === base.name}
                          className="flex items-center gap-1 px-2 py-1 bg-blue-600 hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed text-white text-[10px] font-medium rounded transition-colors shrink-0"
                          title={platformPath ? `Запустить ${base.name}` : 'Платформа не найдена'}
                        >
                          {launching === base.name
                            ? <span className="w-3 h-3 border border-white border-t-transparent rounded-full animate-spin" />
                            : <><Play className="w-2.5 h-2.5" /> Запустить</>
                          }
                        </button>
                      </div>
                    ))
                  )}
                </div>
                {!platformPath && infobases.length > 0 && (
                  <div className="px-3 py-1.5 border-t border-zinc-700 text-[9px] text-amber-400">
                    Платформа 1С не найдена. Включите MCP «1С:Платформа и базы».
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* ── Выбор профиля поиска ── */}
        <button
          onClick={() => { setIsOpen(!isOpen); setShowDatabases(false); }}
          className="flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] transition-all text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50"
        >
          {selectedPid
            ? isMismatch
              ? <AlertTriangle className="w-3 h-3 text-amber-400" />
              : <Search className="w-3 h-3" />
            : <MonitorOff className="w-3 h-3 text-zinc-500" />
          }
          {isBound && <Link className="w-2.5 h-2.5 text-green-400 shrink-0" />}
          <span className={`truncate max-w-[200px] ${
            isMismatch ? 'text-amber-400'
              : activeProfile ? 'text-blue-400'
              : 'text-zinc-500'
          }`}>
            {activeProfile?.name || 'Выберите профиль'}
          </span>
          <ChevronDown className={`w-3 h-3 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </button>

        <span className={`text-[10px] ${isMismatch ? 'text-amber-500' : 'text-zinc-600'}`}>
          {isMismatch ? 'профиль не соответствует' : selectedPid ? 'конфигурация для поиска' : 'ручной выбор'}
        </span>

        {selectedPid && isBound && (
          <button
            onClick={removeBinding}
            className="p-0.5 rounded text-zinc-600 hover:text-red-400 hover:bg-red-500/10 transition-colors"
            title="Отвязать профиль от этого окна"
          >
            <Unlink className="w-3 h-3" />
          </button>
        )}

        {/* ── Индикатор: выберите окно Конфигуратора ── */}
        {!selectedPid && (
          <div className="ml-auto flex items-center gap-1.5 text-[10px] text-amber-500">
            <AlertTriangle className="w-3 h-3" />
            <span>Выберите окно Конфигуратора снизу</span>
          </div>
        )}
      </div>

      {/* ── Дропдаун профилей ── */}
      {isOpen && (
        <div className="absolute bottom-full left-0 mb-1 w-56 bg-zinc-800 border border-zinc-700 rounded-lg shadow-2xl z-50 py-1 animate-in slide-in-from-bottom-2 duration-200">
          <div className="px-3 py-1.5 border-b border-zinc-700 mb-1">
            <span className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider">
              Профиль поиска
            </span>
          </div>
          <div className="max-h-[180px] overflow-y-auto custom-scrollbar">
            {profiles.map(p => (
              <button
                key={p.id}
                onClick={() => handleSelect(p.id)}
                className={`w-full text-left px-3 py-2 text-[12px] flex items-center justify-between transition-colors ${
                  p.id === activeId
                    ? 'bg-blue-500/10 text-blue-400'
                    : 'text-zinc-400 hover:bg-zinc-800/50 hover:text-zinc-200'
                }`}
              >
                <span className="truncate">{p.name}</span>
                {p.id === activeId && <Check className="w-3 h-3 flex-shrink-0 ml-2" />}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
