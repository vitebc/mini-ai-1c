import { useEffect, useState, useCallback } from 'react';
import { Brain, Trash2, FileText, Plus, Save, X, ChevronRight, Eye, Pencil, AlertTriangle } from 'lucide-react';
import { listSkills, getSkill, saveSkill, deleteSkill, SkillFile } from '../../api/skills';

type SkillMode = 'view' | 'edit';

export function SkillsTab() {
    const [skills, setSkills] = useState<SkillFile[]>([]);
    const [selected, setSelected] = useState<SkillFile | null>(null);
    const [editingContent, setEditingContent] = useState('');
    const [dirty, setDirty] = useState(false);
    const [saving, setSaving] = useState(false);
    const [creating, setCreating] = useState(false);
    const [newId, setNewId] = useState('');
    const [mode, setMode] = useState<SkillMode>('view');
    const [confirmDelete, setConfirmDelete] = useState<SkillFile | null>(null);

    const load = useCallback(async () => {
        const items = await listSkills();
        setSkills(items);
    }, []);

    useEffect(() => { load(); }, [load]);

    const handleSelect = async (s: SkillFile) => {
        if (dirty && !confirm('Сохранить изменения перед переключением?')) return;
        const full = await getSkill(s.id);
        setSelected(full);
        setEditingContent(full.content);
        setDirty(false);
        setCreating(false);
    };

    const handleSave = async () => {
        if (!selected) return;
        setSaving(true);
        try {
            const updated = await saveSkill(selected.id, editingContent);
            setSelected(updated);
            setEditingContent(updated.content);
            setDirty(false);
            await load();
        } finally {
            setSaving(false);
        }
    };

    const performDelete = async (id: string) => {
        await deleteSkill(id);
        if (selected?.id === id) {
            setSelected(null);
            setEditingContent('');
            setMode('view');
        }
        setConfirmDelete(null);
        await load();
    };

    const handleCreate = async () => {
        if (!newId.trim()) return;
        const template = `---\nname: ${newId}\ndescription: \n---\n`;
        await saveSkill(newId.trim(), template);
        setNewId('');
        setCreating(false);
        await load();
        const created = await getSkill(newId.trim());
        setSelected(created);
        setEditingContent(created.content);
        setMode('edit');
    };

    return (
        <div className="flex h-full bg-zinc-800">
            {/* Left sidebar — skill list */}
            <div className="w-64 shrink-0 border-r border-zinc-800 flex flex-col bg-zinc-900/50">
                <div className="flex items-center justify-between px-3 py-2.5 border-b border-zinc-800">
                    <span className="text-xs font-semibold text-zinc-400 uppercase tracking-wider flex items-center gap-1.5">
                        <Brain className="w-3.5 h-3.5" /> Скиллы
                    </span>
                    
                </div>

                {creating && (
                    <div className="p-2 border-b border-zinc-800">
                        <input
                            value={newId}
                            onChange={e => setNewId(e.target.value)}
                            placeholder="ID скилла (например: my-skill)"
                            className="w-full px-2 py-1.5 text-xs bg-[var(--input-bg)] border border-zinc-700 rounded text-zinc-300 placeholder-zinc-500 focus:outline-none focus:border-blue-500 mb-1.5"
                            onKeyDown={e => { if (e.key === 'Enter') handleCreate(); if (e.key === 'Escape') setCreating(false); }}
                            autoFocus
                        />
                        <div className="flex gap-1">
                            <button onClick={handleCreate} className="px-2 py-1 text-[10px] bg-blue-600 text-white rounded hover:bg-blue-500">Создать</button>
                            <button onClick={() => {setCreating(false); setNewId('')}} className="px-2 py-1 text-[10px] bg-zinc-700 text-zinc-400 rounded hover:bg-zinc-600">Отмена</button>
                        </div>
                    </div>
                )}

                <div className="flex-1 overflow-y-auto custom-scrollbar py-1">
                    {skills.length === 0 && (
                        <div className="px-3 py-6 text-center text-xs text-zinc-600">Нет скиллов</div>
                    )}
                    {skills.map(s => (
                        <div
                            key={s.id}
                            onClick={() => handleSelect(s)}
                            className={`group flex items-center gap-2 px-3 py-2 mx-1 rounded-md cursor-pointer text-sm transition-colors ${
                                selected?.id === s.id
                                    ? 'bg-blue-500/10 text-blue-400'
                                    : 'text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200'
                            }`}
                        >
                            <FileText className="w-3.5 h-3.5 shrink-0" />
                            <div className="flex-1 min-w-0">
                                <div className="truncate text-xs font-medium">{s.name || s.id}</div>
                                {s.name && <div className="truncate text-[10px] text-zinc-600">{s.id}</div>}
                            </div>
                            <button
                                onClick={(e) => { e.stopPropagation(); setConfirmDelete(s); }}
                                className="shrink-0 p-0.5 rounded opacity-0 group-hover:opacity-100 hover:bg-red-500/20 text-zinc-500 hover:text-red-400 transition-all"
                                title="Удалить"
                            >
                                <Trash2 className="w-3 h-3" />
                            </button>
                        </div>
                    ))}
                </div>
            </div>

            {/* Right panel — markdown editor */}
            <div className="flex-1 flex flex-col min-w-0">
                {selected || creating ? (
                    <>
                        {/* Toolbar */}
                        <div className="flex items-center justify-between px-4 py-2.5 border-b border-zinc-800 bg-zinc-900/50 shrink-0">
                            <div className="flex items-center gap-2 text-sm text-zinc-300">
                                <button
                                    onClick={() => { setCreating(true); setSelected(null); setMode('edit'); }}
                                    className="p-1 rounded hover:bg-zinc-700 text-zinc-400 hover:text-zinc-200 transition-colors"
                                    title="Новый скилл">
                                        <div className="flex items-center gap-2 text-sm text-zinc-300">
                                        {/*<Plus className="w-3.5 h-3.5" />*/}
                                        <Brain className="w-4 h-4 text-zinc-500" />
                                        {selected?.name || newId || 'Добавить скилл'}
                                        </div>
                                </button>
                                
                                {dirty && <span className="text-[10px] text-amber-400">* изменено</span>}
                            </div>
                            <div className="flex items-center gap-2">
                                <div className="flex items-center rounded-lg bg-zinc-800 p-0.5">
                                    {selected && mode === 'edit' && (
                                    <button
                                        onClick={handleSave}
                                        disabled={saving || !dirty}
                                        className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors ${
                                            dirty
                                                ? 'bg-blue-600 text-white hover:bg-blue-500'
                                                : 'bg-zinc-800 text-zinc-600 cursor-not-allowed'
                                        }`}
                                    >
                                        <Save className="w-3.5 h-3.5" />
                                        {saving ? 'Сохранение...' : 'Сохранить'}
                                    </button>
                                )}
                                    <button
                                        onClick={() => setMode('view')}
                                        className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-md transition-colors ${
                                            mode === 'view'
                                                ? 'bg-zinc-700 text-zinc-100 shadow'
                                                : 'text-zinc-400 hover:text-zinc-200'
                                        }`}
                                        title="Просмотр"
                                    >
                                        <Eye className="w-3.5 h-3.5" />
                                        Просмотр
                                    </button>
                                    <button
                                        onClick={() => setMode('edit')}
                                        className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-md transition-colors ${
                                            mode === 'edit'
                                                ? 'bg-zinc-700 text-zinc-100 shadow'
                                                : 'text-zinc-400 hover:text-zinc-200'
                                        }`}
                                        title="Редактор"
                                    >
                                        <Pencil className="w-3.5 h-3.5" />
                                        Редактор
                                    </button>
                                </div>
                                
                            </div>
                        </div>

                        {/* Editor */}
                        {mode === 'edit' ? (
                            <div className="flex-1 min-h-0">
                                <textarea
                                    value={editingContent}
                                    onChange={e => { setEditingContent(e.target.value); setDirty(true); }}
                                    className="w-full h-full p-4 text-sm font-mono bg-[var(--input-bg)] text-zinc-300 resize-none focus:outline-none custom-scrollbar bg-zinc-600"
                                    spellCheck={false}
                                />
                            </div>
                        ) : (
                            <div className="flex-1 p-6 overflow-y-auto custom-scrollbar bg-zinc-800">
                                <MarkdownPreview markdown={editingContent} />
                            </div>
                        )}
                    </>
                ) : (
                    <div className="flex-1 flex items-center justify-center">
                        <div className="text-center">
                            <Brain className="w-12 h-12 text-zinc-700 mx-auto mb-3" />
                            <p className="text-sm text-zinc-600">Выберите скилл из списка</p>
                            <p className="text-xs text-zinc-700 mt-1">или создайте новый</p>
                        </div>
                    </div>
                )}
            </div>

            {confirmDelete && (
                <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
                    <div className="w-80 rounded-xl border border-zinc-700 bg-zinc-900 p-5 shadow-2xl">
                        <div className="flex items-center gap-2 mb-3">
                            <AlertTriangle className="w-4 h-4 text-red-400 shrink-0" />
                            <span className="text-sm font-semibold text-zinc-100">Удалить скилл</span>
                        </div>
                        <p className="text-xs text-zinc-400 leading-relaxed mb-5">
                            Вы уверены, что хотите удалить скилл{' '}
                            <span className="text-zinc-200 font-medium">"{confirmDelete.name || confirmDelete.id}"</span>?
                            Это действие необратимо.
                        </p>
                        <div className="flex justify-end gap-2">
                            <button
                                onClick={() => setConfirmDelete(null)}
                                className="px-3 py-1.5 text-xs font-medium rounded-lg bg-zinc-800 text-zinc-300 hover:bg-zinc-700 transition-colors"
                            >
                                Отмена
                            </button>
                            <button
                                onClick={() => performDelete(confirmDelete.id)}
                                className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg bg-red-600 text-white hover:bg-red-500 transition-colors"
                            >
                                <Trash2 className="w-3.5 h-3.5" />
                                Удалить
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

// Simple markdown preview (no external deps)
function MarkdownPreview({ markdown }: { markdown: string }) {
    const lines = markdown.split('\n');
    const html = lines.map(line => {
        if (line.startsWith('# ')) return `<h1 class="text-lg font-bold text-zinc-100 mt-4 mb-2">${escapeHtml(line.slice(2))}</h1>`;
        if (line.startsWith('## ')) return `<h2 class="text-base font-semibold text-zinc-200 mt-3 mb-1.5">${escapeHtml(line.slice(3))}</h2>`;
        if (line.startsWith('### ')) return `<h3 class="text-sm font-medium text-zinc-300 mt-2 mb-1">${escapeHtml(line.slice(4))}</h3>`;
        if (line.startsWith('- ')) return `<li class="text-xs text-zinc-400 ml-4 list-disc">${escapeHtml(line.slice(2))}</li>`;
        if (line.startsWith('> ')) return `<blockquote class="border-l-2 border-zinc-600 pl-3 text-xs text-zinc-500 italic my-1">${escapeHtml(line.slice(2))}</blockquote>`;
        if (line.startsWith('```')) return '';
        if (line.trim() === '') return '<div class="h-2"></div>';
        // Inline code
        const withCode = line.replace(/`([^`]+)`/g, '<code class="bg-zinc-800 text-zinc-300 px-1 rounded text-[11px] font-mono">$1</code>');
        // Bold
        const withBold = withCode.replace(/\*\*([^*]+)\*\*/g, '<strong class="text-zinc-200 font-semibold">$1</strong>');
        return `<p class="text-xs text-zinc-400 leading-relaxed">${withBold}</p>`;
    }).join('\n');

    return <div className="prose prose-invert prose-sm max-w-none" dangerouslySetInnerHTML={{ __html: html }} />;
}

function escapeHtml(text: string): string {
    return text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}
