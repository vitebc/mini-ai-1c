import { useState, useMemo, useCallback, useEffect } from 'react';
import { useChat, ChatSession } from '../../contexts/ChatContext';
import { ChevronRight, MessageSquare, MessageSquarePlus, FolderClosed, FolderOpen, FileText, X } from 'lucide-react';

interface SubGroup {
  label: string;
  sessions: ChatSession[];
}

interface Group {
  configName: string;
  children: SubGroup[];
  flatSessions: ChatSession[];
}

function useExpanded(key: string): [boolean, () => void] {
  const [expanded, setExpanded] = useState(() => {
    try { return localStorage.getItem(`sessions_expanded_${key}`) === 'true'; } catch { return true; }
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

function SessionItem({ session, isActive, onSwitch, onDelete }: { session: ChatSession; isActive: boolean; onSwitch: (id: string) => void; onDelete: (id: string) => void }) {
  const [showDelete, setShowDelete] = useState(false);

  return (
    <div
      className={`group flex items-center gap-1.5 px-2 py-1.5 mx-1 rounded-md cursor-pointer text-[12px] transition-colors ${
        isActive
          ? 'bg-emerald-500/10 text-emerald-400'
          : 'text-zinc-400 hover:bg-[#1f1f23] hover:text-zinc-200'
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
          className="shrink-0 p-0.5 rounded hover:bg-red-500/20 text-zinc-500 hover:text-red-400 transition-colors"
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
  const [isOpen, setIsOpen] = useState(() => {
    try { return localStorage.getItem('sessions_panel_open') === 'true'; } catch { return false; }
  });

  useEffect(() => {
    try { localStorage.setItem('sessions_panel_open', String(isOpen)); } catch {}
  }, [isOpen]);

  const groups = useMemo(() => {
    const configMap = new Map<string, { flatSessions: ChatSession[]; objectMap: Map<string, ChatSession[]> }>();

    for (const s of sessions) {
      const config = s.configName || '';
      if (!configMap.has(config)) {
        configMap.set(config, { flatSessions: [], objectMap: new Map() });
      }
      const entry = configMap.get(config)!;

      if (s.objectPath) {
        if (!entry.objectMap.has(s.objectPath)) {
          entry.objectMap.set(s.objectPath, []);
        }
        entry.objectMap.get(s.objectPath)!.push(s);
      } else {
        entry.flatSessions.push(s);
      }
    }

    const result: Group[] = [];

    for (const [configName, entry] of configMap) {
      const children: SubGroup[] = [];
      for (const [objectPath, obSessions] of entry.objectMap) {
        children.push({
          label: objectPath,
          sessions: obSessions.sort((a, b) => b.updatedAt - a.updatedAt),
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

  return (
    <div className="flex min-h-0">
      <div className={`transition-all duration-200 ease-out overflow-hidden flex flex-col ${isOpen ? 'w-[240px] min-w-0' : 'w-0'}`}>
        <div className="w-[240px] flex-1 border-r border-[#27272a] bg-[#0d0d10] flex flex-col shrink-0">
          <div className="flex items-center justify-between px-3 py-2.5 border-b border-[#27272a]">
            <span className="text-[10px] font-bold text-zinc-500 uppercase tracking-wider">Сессии</span>
            <button
              onClick={handleNewChat}
              className="p-1 rounded hover:bg-[#27272a] text-zinc-400 hover:text-zinc-200 transition-colors"
              title="Новый чат"
            >
              <MessageSquarePlus className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="flex-1 overflow-y-auto custom-scrollbar py-1.5 space-y-0.5">
            {groups.length === 0 ? (
              <div className="px-3 py-6 text-center text-[11px] text-zinc-600">
                Нет сессий
              </div>
            ) : groups.map(group => (
              <ConfigGroup
                key={group.configName}
                group={group}
                activeSessionId={activeSessionId}
                onSwitch={switchChat}
                onDelete={deleteChat}
              />
            ))}
          </div>
        </div>
      </div>

      <div className="flex flex-col shrink-0">
        {!isOpen && (
          <div className="w-[3px] flex-1 bg-gradient-to-b from-transparent via-zinc-700/30 to-transparent" />
        )}
        <button
          onClick={() => setIsOpen(v => !v)}
          className={`flex items-center justify-center w-6 h-16 my-auto rounded-r-md transition-all shrink-0 cursor-pointer ${
            isOpen
              ? 'bg-[#1f1f23] text-zinc-300 hover:bg-[#27272a] shadow-sm'
              : 'bg-[#141418] text-zinc-500 hover:text-zinc-200 hover:bg-[#1f1f23] hover:w-7'
          }`}
          title={isOpen ? 'Скрыть панель сессий' : 'Показать панель сессий'}
        >
          <ChevronRight className={`w-3.5 h-3.5 transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`} />
        </button>
        {!isOpen && (
          <div className="w-[3px] flex-1 bg-gradient-to-b from-zinc-700/30 via-transparent to-zinc-700/30" />
        )}
      </div>
    </div>
  );
}

function ConfigGroup({ group, activeSessionId, onSwitch, onDelete }: { group: Group; activeSessionId: string | null; onSwitch: (id: string) => void; onDelete: (id: string) => void }) {
  const [expanded, toggle] = useExpanded(`config_${group.configName}`);

  const allSessionIds = useMemo(() => {
    const ids = new Set<string>();
    for (const s of group.flatSessions) ids.add(s.id);
    for (const child of group.children) {
      for (const s of child.sessions) ids.add(s.id);
    }
    return ids;
  }, [group]);

  const hasActive = activeSessionId && allSessionIds.has(activeSessionId);

  return (
    <div>
      <button
        onClick={toggle}
        className={`flex items-center gap-1.5 w-full px-3 py-1.5 text-[11px] font-medium transition-colors ${
          hasActive ? 'text-emerald-400' : 'text-zinc-400 hover:text-zinc-200'
        }`}
      >
        {expanded ? <FolderOpen className="w-3 h-3 shrink-0" /> : <FolderClosed className="w-3 h-3 shrink-0" />}
        <ChevronRight className={`w-2.5 h-2.5 shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
        <span className="truncate">{group.configName}</span>
        <span className="text-[10px] text-zinc-600 ml-auto">{allSessionIds.size}</span>
      </button>

      {expanded && (
        <div className="ml-1">
          {group.flatSessions.map(s => (
            <SessionItem key={s.id} session={s} isActive={s.id === activeSessionId} onSwitch={onSwitch} onDelete={onDelete} />
          ))}

          {group.children.map(child => (
            <ObjectGroup
              key={child.label}
              label={child.label}
              sessions={child.sessions}
              activeSessionId={activeSessionId}
              onSwitch={onSwitch}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function ObjectGroup({ label, sessions, activeSessionId, onSwitch, onDelete }: { label: string; sessions: ChatSession[]; activeSessionId: string | null; onSwitch: (id: string) => void; onDelete: (id: string) => void }) {
  const [expanded, toggle] = useExpanded(`obj_${label}`);

  const hasActive = sessions.some(s => s.id === activeSessionId);

  return (
    <div>
      <button
        onClick={toggle}
        className={`flex items-center gap-1.5 w-full px-3 py-1 pl-5 text-[11px] font-medium transition-colors ${
          hasActive ? 'text-emerald-400' : 'text-zinc-500 hover:text-zinc-300'
        }`}
      >
        <ChevronRight className={`w-2.5 h-2.5 shrink-0 transition-transform ${expanded ? 'rotate-90' : ''}`} />
        <FileText className="w-3 h-3 shrink-0" />
        <span className="truncate">{label}</span>
        <span className="text-[10px] text-zinc-600 ml-auto">{sessions.length}</span>
      </button>

      {expanded && (
        <div>
          {sessions.map(s => (
            <SessionItem key={s.id} session={s} isActive={s.id === activeSessionId} onSwitch={onSwitch} onDelete={onDelete} />
          ))}
        </div>
      )}
    </div>
  );
}
