import { useState, useMemo, useCallback, useEffect, useRef } from 'react';
import { useChat, ChatSession } from '../../contexts/ChatContext';
import { useSettings } from '../../contexts/SettingsContext';
import { ChevronRight, MessageSquare, MessageSquarePlus, FolderClosed, FolderOpen, FileText, X } from 'lucide-react';

const PANEL_MIN = 280;
const PANEL_MAX = 480;
const PANEL_WIDTH_KEY = 'sessions_panel_width';

const KNOWN_MODULE_SUFFIXES = [
  'МодульМенеджера', 'МодульОбъекта', 'МодульФормы', 'МодульНабораЗаписей',
  'МодульПриложения', 'МодульВнешнегоСоединения', 'МодульСеанса', 'Модуль', 'Форма',
];

interface ModuleGroupData {
  moduleName: string;
  sessions: ChatSession[];
}

interface ObjectGroupData {
  label: string;
  modules: ModuleGroupData[];
  flatSessions: ChatSession[];
}

interface Group {
  configName: string;
  children: ObjectGroupData[];
  flatSessions: ChatSession[];
}

function splitModule(path: string, givenModule?: string): { basePath: string; moduleName: string | null } {
  if (givenModule) {
    if (path.endsWith('.' + givenModule)) {
      return { basePath: path.slice(0, -givenModule.length - 1), moduleName: givenModule };
    }
    return { basePath: path, moduleName: givenModule };
  }
  for (const suffix of KNOWN_MODULE_SUFFIXES) {
    if (path.endsWith('.' + suffix)) {
      return { basePath: path.slice(0, -suffix.length - 1), moduleName: suffix };
    }
  }
  return { basePath: path, moduleName: null };
}

function useExpanded(key: string): [boolean, () => void] {
  const [expanded, setExpanded] = useState(() => {
    try { return localStorage.getItem(`sessions_expanded_${key}`) !== 'false'; } catch { return true; }
  });
  const toggle = useCallback(() => {
    setExpanded(v => {
      const next = !v;
      try { localStorage.setItem(`sessions_expanded_${key}`, String(next)); } catch {}
      return next;
    });
  }, [key]);
  return [expanded, toggle];
}

function SessionItem({ session, isActive, onSwitch, onDelete, isLight }: { session: ChatSession; isActive: boolean; onSwitch: (id: string) => void; onDelete: (id: string) => void; isLight: boolean }) {
  const [showDelete, setShowDelete] = useState(false);

  return (
    <div
      className={`group flex items-center gap-1.5 px-2 py-1.5 mx-1 rounded-md cursor-pointer text-[12px] transition-colors ${
        isActive
          ? isLight ? 'bg-emerald-100 text-emerald-700' : 'bg-emerald-500/10 text-emerald-400'
          : isLight ? 'text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800' : 'text-zinc-400 hover:bg-[#1f1f23] hover:text-zinc-200'
      }`}
      onClick={() => onSwitch(session.id)}
      onMouseEnter={() => setShowDelete(true)}
      onMouseLeave={() => setShowDelete(false)}
    >
      <MessageSquare className="w-3 h-3 shrink-0" />
      <span className="truncate flex-1">{session.title}</span>
      {showDelete && (
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(session.id); }}
          className={`shrink-0 p-0.5 rounded transition-colors ${
            isLight ? 'text-zinc-400 hover:bg-red-100 hover:text-red-600' : 'text-zinc-500 hover:bg-red-500/20 hover:text-red-400'
          }`}
          title="Удалить"
        >
          <X className="w-3 h-3" />
        </button>
      )}
    </div>
  );
}

export function SessionsPanel() {
  const { sessions, activeSessionId, switchChat, deleteChat, createNewChat } = useChat();
  const { settings } = useSettings();
  const isLight = settings?.theme === 'light';

  const [isOpen, setIsOpen] = useState(() => {
    try { return localStorage.getItem('sessions_panel_open') !== 'false'; } catch { return true; }
  });
  const [panelWidth, setPanelWidth] = useState(() => {
    try {
      const saved = localStorage.getItem(PANEL_WIDTH_KEY);
      return saved ? Math.max(PANEL_MIN, Math.min(PANEL_MAX, Number(saved))) : PANEL_MIN;
    } catch { return PANEL_MIN; }
  });

  const resizeRef = useRef<{ startX: number; startW: number } | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  useEffect(() => {
    try { localStorage.setItem('sessions_panel_open', String(isOpen)); } catch {}
  }, [isOpen]);

  useEffect(() => {
    try { localStorage.setItem(PANEL_WIDTH_KEY, String(panelWidth)); } catch {}
  }, [panelWidth]);

  const groups = useMemo(() => {
    const configMap = new Map<string, { flatSessions: ChatSession[]; objectMap: Map<string, { moduleMap: Map<string, ChatSession[]>; flatSessions: ChatSession[] }> }>();

    for (const s of sessions) {
      const config = s.configName || '';
      if (!configMap.has(config)) {
        configMap.set(config, { flatSessions: [], objectMap: new Map() });
      }
      const entry = configMap.get(config)!;

      if (s.objectPath) {
        const { basePath, moduleName } = splitModule(s.objectPath, s.moduleType);

        if (!entry.objectMap.has(basePath)) {
          entry.objectMap.set(basePath, { moduleMap: new Map(), flatSessions: [] });
        }
        const obj = entry.objectMap.get(basePath)!;

        if (moduleName) {
          if (!obj.moduleMap.has(moduleName)) {
            obj.moduleMap.set(moduleName, []);
          }
          obj.moduleMap.get(moduleName)!.push(s);
        } else {
          obj.flatSessions.push(s);
        }
      } else {
        entry.flatSessions.push(s);
      }
    }

    const result: Group[] = [];

    for (const [configName, entry] of configMap) {
      const children: ObjectGroupData[] = [];

      for (const [basePath, obj] of entry.objectMap) {
        const modules: ModuleGroupData[] = [];

        for (const [mn, obSessions] of obj.moduleMap) {
          modules.push({ moduleName: mn, sessions: obSessions.sort((a, b) => b.updatedAt - a.updatedAt) });
        }
        modules.sort((a, b) => a.moduleName.localeCompare(b.moduleName));

        children.push({
          label: basePath,
          modules,
          flatSessions: obj.flatSessions.sort((a, b) => b.updatedAt - a.updatedAt),
        });
      }
      children.sort((a, b) => a.label.localeCompare(b.label));

      result.push({
        configName: configName || 'Без конфигурации',
        children,
        flatSessions: entry.flatSessions.sort((a, b) => b.updatedAt - a.updatedAt),
      });
    }

    result.sort((a, b) => {
      if (a.configName === 'Без конфигурации') return 1;
      if (b.configName === 'Без конфигурации') return -1;
      return a.configName.localeCompare(b.configName);
    });

    return result;
  }, [sessions]);

  const handleNewChat = useCallback(() => {
    createNewChat();
    if (!isOpen) setIsOpen(true);
  }, [createNewChat, isOpen]);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = panelWidth;

    const onMouseMove = (ev: MouseEvent) => {
      const newW = Math.max(PANEL_MIN, Math.min(PANEL_MAX, startW + (ev.clientX - startX)));
      setPanelWidth(newW);
    };
    const onMouseUp = () => {
      setIsResizing(false);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
    setIsResizing(true);
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);
  }, [panelWidth]);

  return (
    <div className="flex min-h-0">
      <div className={`overflow-hidden flex flex-col shrink-0 ${isOpen ? 'opacity-100' : 'w-0 opacity-0'}`}
        style={{
          width: isOpen ? panelWidth : 0,
          minWidth: isOpen ? PANEL_MIN : 0,
          maxWidth: isOpen ? PANEL_MAX : 0,
          transition: isResizing ? 'none' : 'width 0.2s ease-out, opacity 0.15s ease-out',
        }}
      >
        <div
          className={`flex-1 flex flex-col shrink-0 border-r ${
            isLight ? 'border-zinc-200 bg-white' : 'border-[#27272a] bg-[#0d0d10]'
          }`}
          style={{ width: panelWidth }}
        >
          <div className={`flex items-center justify-between px-3 py-2.5 border-b ${
            isLight ? 'border-zinc-200' : 'border-[#27272a]'
          }`}>
            <span className={`text-[10px] font-bold uppercase tracking-wider ${isLight ? 'text-zinc-500' : 'text-zinc-500'}`}>Сессии</span>
            <button
              onClick={handleNewChat}
              className={`p-1 rounded transition-colors ${
                isLight ? 'hover:bg-zinc-100 text-zinc-500 hover:text-zinc-700' : 'hover:bg-[#27272a] text-zinc-400 hover:text-zinc-200'
              }`}
              title="Новый чат"
            >
              <MessageSquarePlus className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto custom-scrollbar py-1.5 space-y-0.5">
            {groups.length === 0 ? (
              <div className={`px-3 py-6 text-center text-[11px] ${isLight ? 'text-zinc-400' : 'text-zinc-600'}`}>
                Нет сессий
              </div>
            ) : groups.map(group => (
              <ConfigGroup
                key={group.configName}
                group={group}
                activeSessionId={activeSessionId}
                onSwitch={switchChat}
                onDelete={deleteChat}
                isLight={isLight}
              />
            ))}
          </div>
        </div>
      </div>

      {isOpen && (
        <div
          onMouseDown={handleResizeStart}
          className="relative w-1.5 shrink-0 cursor-col-resize hover:bg-blue-500/30 transition-colors group flex items-center justify-center"
          title="Изменить размер"
        >
          <div className={`w-0.5 h-8 rounded-full opacity-0 group-hover:opacity-100 transition-opacity ${
            isLight ? 'bg-zinc-400 group-hover:bg-blue-500' : 'bg-zinc-700 group-hover:bg-blue-400'
          }`} />
        </div>
      )}

      <button
        onClick={() => setIsOpen(v => !v)}
        className={`flex items-center justify-center w-6 h-16 my-auto rounded-r-md transition-all shrink-0 cursor-pointer ${
          isOpen
            ? isLight
              ? 'bg-[#1f1f23] text-zinc-600 hover:bg-[#27272a] hover:text-zinc-800 shadow-sm'
              : 'bg-[#1f1f23] text-zinc-300 hover:bg-[#27272a] shadow-sm'
            : isLight
              ? 'bg-[#1f1f23] text-zinc-400 hover:text-zinc-600 hover:bg-[#27272a]'
              : 'bg-[#1f1f23] text-zinc-500 hover:text-zinc-200 hover:bg-[#27272a]'
        }`}
        title={isOpen ? 'Скрыть панель сессий' : 'Показать панель сессий'}
      >
        <ChevronRight className={`w-3.5 h-3.5 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`} />
      </button>
    </div>
  );
}

function ConfigGroup({ group, activeSessionId, onSwitch, onDelete, isLight }: { group: Group; activeSessionId: string | null; onSwitch: (id: string) => void; onDelete: (id: string) => void; isLight: boolean }) {
  const [expanded, toggle] = useExpanded(`config_${group.configName}`);

  const allSessionIds = useMemo(() => {
    const ids = new Set<string>();
    for (const s of group.flatSessions) ids.add(s.id);
    for (const child of group.children) {
      for (const s of child.flatSessions) ids.add(s.id);
      for (const mod of child.modules) {
        for (const s of mod.sessions) ids.add(s.id);
      }
    }
    return ids;
  }, [group]);

  const hasActive = activeSessionId && allSessionIds.has(activeSessionId);

  return (
    <div>
      <button
        onClick={toggle}
        className={`flex items-center gap-1.5 w-full px-3 py-1.5 text-[11px] font-medium transition-colors ${
          hasActive
            ? isLight ? 'text-emerald-600' : 'text-emerald-400'
            : isLight ? 'text-zinc-500 hover:text-zinc-700' : 'text-zinc-400 hover:text-zinc-200'
        }`}
      >
        {expanded ? <FolderOpen className="w-3 h-3 shrink-0" /> : <FolderClosed className="w-3 h-3 shrink-0" />}
        <ChevronRight className={`w-2.5 h-2.5 shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
        <span className="truncate">{group.configName}</span>
        <span className={`text-[10px] ml-auto ${isLight ? 'text-zinc-400' : 'text-zinc-600'}`}>{allSessionIds.size}</span>
      </button>

      {expanded && (
        <div className="ml-1">
          {group.flatSessions.length > 0 && (
            <div className="pl-3">
              {group.flatSessions.map(s => (
                <SessionItem key={s.id} session={s} isActive={s.id === activeSessionId} onSwitch={onSwitch} onDelete={onDelete} isLight={isLight} />
              ))}
            </div>
          )}

          {group.children.map(child => (
            <ObjectGroup
              key={child.label}
              label={child.label}
              modules={child.modules}
              flatSessions={child.flatSessions}
              activeSessionId={activeSessionId}
              onSwitch={onSwitch}
              onDelete={onDelete}
              isLight={isLight}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ObjectGroup({ label, modules, flatSessions, activeSessionId, onSwitch, onDelete, isLight }: { label: string; modules: ModuleGroupData[]; flatSessions: ChatSession[]; activeSessionId: string | null; onSwitch: (id: string) => void; onDelete: (id: string) => void; isLight: boolean }) {
  const [expanded, toggle] = useExpanded(`obj_${label}`);

  const allIds = useMemo(() => {
    const ids = new Set<string>();
    for (const s of flatSessions) ids.add(s.id);
    for (const mod of modules) {
      for (const s of mod.sessions) ids.add(s.id);
    }
    return ids;
  }, [modules, flatSessions]);

  const hasActive = activeSessionId && allIds.has(activeSessionId);

  return (
    <div>
      <button
        onClick={toggle}
        className={`flex items-center gap-1.5 w-full px-3 py-1 pl-5 text-[11px] font-medium transition-colors ${
          hasActive
            ? isLight ? 'text-emerald-600' : 'text-emerald-400'
            : isLight ? 'text-zinc-500 hover:text-zinc-700' : 'text-zinc-500 hover:text-zinc-300'
        }`}
      >
        <ChevronRight className={`w-2.5 h-2.5 shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
        <FileText className="w-3 h-3 shrink-0" />
        <span className="truncate">{label}</span>
        <span className={`text-[10px] ml-auto ${isLight ? 'text-zinc-400' : 'text-zinc-600'}`}>{allIds.size}</span>
      </button>

      {expanded && (
        <div>
          {flatSessions.map(s => (
            <SessionItem key={s.id} session={s} isActive={s.id === activeSessionId} onSwitch={onSwitch} onDelete={onDelete} isLight={isLight} />
          ))}

          {modules.map(mod => (
            <ModuleGroup
              key={mod.moduleName}
              moduleName={mod.moduleName}
              sessions={mod.sessions}
              activeSessionId={activeSessionId}
              onSwitch={onSwitch}
              onDelete={onDelete}
              isLight={isLight}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ModuleGroup({ moduleName, sessions, activeSessionId, onSwitch, onDelete, isLight }: { moduleName: string; sessions: ChatSession[]; activeSessionId: string | null; onSwitch: (id: string) => void; onDelete: (id: string) => void; isLight: boolean }) {
  const [expanded, toggle] = useExpanded(`mod_${moduleName}`);

  const hasActive = sessions.some(s => s.id === activeSessionId);

  return (
    <div>
      <button
        onClick={toggle}
        className={`flex items-center gap-1.5 w-full px-3 py-1 pl-8 text-[11px] font-medium transition-colors ${
          hasActive
            ? isLight ? 'text-emerald-600' : 'text-emerald-400'
            : isLight ? 'text-zinc-400 hover:text-zinc-600' : 'text-zinc-500 hover:text-zinc-300'
        }`}
      >
        <ChevronRight className={`w-2 h-2 shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
        <span className="truncate">{moduleName}</span>
        <span className={`text-[10px] ml-auto ${isLight ? 'text-zinc-400' : 'text-zinc-600'}`}>{sessions.length}</span>
      </button>

      {expanded && (
        <div>
          {sessions.map(s => (
            <SessionItem key={s.id} session={s} isActive={s.id === activeSessionId} onSwitch={onSwitch} onDelete={onDelete} isLight={isLight} />
          ))}
        </div>
      )}
    </div>
  );
}
