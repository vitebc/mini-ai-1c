import { useState, useEffect, useRef, useCallback } from 'react';
import { Search, ChevronDown, Check, Link, Unlink } from 'lucide-react';
import { useSettings } from '../../contexts/SettingsContext';
import { useConfigurator } from '../../contexts/ConfiguratorContext';
import {
  BUILTIN_1C_SEARCH_ID,
  normalizeSearchProfiles,
  buildSearchEnv,
} from '../../utils/searchProfiles';

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

export function SearchProfileBar() {
  const { settings, updateSettings } = useSettings();
  const { selectedPid } = useConfigurator();
  const [isOpen, setIsOpen] = useState(false);
  const [isBound, setIsBound] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const searchServer = settings?.mcp_servers.find(
    s => s.id === BUILTIN_1C_SEARCH_ID && s.enabled,
  );
  const { profiles, activeId } = searchServer
    ? normalizeSearchProfiles(searchServer)
    : { profiles: [], activeId: '' };
  const activeProfile = profiles.find(p => p.id === activeId) || profiles[0];

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
    if (!selectedPid || !searchServer || profiles.length === 0) {
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
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Hide completely if MCP search is disabled
  if (!searchServer || profiles.length === 0) return null;

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

  return (
    <div className="max-w-4xl mx-auto w-full px-1 mb-2 relative" ref={dropdownRef}>
      <div className="flex items-center gap-2">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className={`flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] transition-all ${
            !selectedPid
              ? 'text-red-400 hover:text-red-300 hover:bg-red-500/10'
              : 'text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50'
          }`}
        >
          <Search className="w-3 h-3" />
          {selectedPid ? (
            <>
              {isBound && <Link className="w-2.5 h-2.5 text-green-400 shrink-0" />}
              <span className="truncate max-w-[200px] text-blue-400">{activeProfile?.name || 'Нет профиля'}</span>
            </>
          ) : (
            <span className="text-red-400 font-medium">Профиль не выбран</span>
          )}
          <ChevronDown className={`w-3 h-3 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </button>
        <span className="text-[10px] text-zinc-600">конфигурация для поиска</span>
        {selectedPid && isBound && (
          <button
            onClick={removeBinding}
            className="p-0.5 rounded text-zinc-600 hover:text-red-400 hover:bg-red-500/10 transition-colors"
            title="Отвязать профиль от этого окна"
          >
            <Unlink className="w-3 h-3" />
          </button>
        )}
      </div>
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
