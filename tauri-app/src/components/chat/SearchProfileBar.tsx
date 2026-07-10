import { useState, useEffect, useRef } from 'react';
import { Search, ChevronDown, Check } from 'lucide-react';
import { useSettings } from '../../contexts/SettingsContext';
import {
  BUILTIN_1C_SEARCH_ID,
  normalizeSearchProfiles,
  buildSearchEnv,
} from '../../utils/searchProfiles';

export function SearchProfileBar() {
  const { settings, updateSettings } = useSettings();
  const [isOpen, setIsOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const searchServer = settings?.mcp_servers.find(
    s => s.id === BUILTIN_1C_SEARCH_ID && s.enabled,
  );
  const { profiles, activeId } = searchServer
    ? normalizeSearchProfiles(searchServer)
    : { profiles: [], activeId: '' };
  const activeProfile = profiles.find(p => p.id === activeId) || profiles[0];

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

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
    setIsOpen(false);
  };

  return (
    <div className="max-w-4xl mx-auto w-full px-1 mb-2 relative" ref={dropdownRef}>
      <div className="flex items-center gap-2">
        <button
          onClick={() => setIsOpen(!isOpen)}
          className="flex items-center gap-1.5 px-2 py-1 rounded-md text-[11px] text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50 transition-all"
        >
          <Search className="w-3 h-3" />
          <span className="truncate max-w-[200px]">{activeProfile?.name || 'Нет профиля'}</span>
          <ChevronDown className={`w-3 h-3 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
        </button>
        <span className="text-[10px] text-zinc-600">конфигурация для поиска</span>
      </div>
      {isOpen && (
        <div className="absolute bottom-full left-0 mb-1 w-56 bg-[#1f1f23] border border-[#27272a] rounded-lg shadow-2xl z-50 py-1 animate-in slide-in-from-bottom-2 duration-200">
          <div className="px-3 py-1.5 border-b border-[#27272a] mb-1">
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
