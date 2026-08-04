import { Check } from 'lucide-react';
import { useSettings } from '@/contexts/SettingsContext';

interface FooterProps {
    onApply: () => void;
    isApplying: boolean;
    modifiedCode: string;
}

export function Footer({
    onApply,
    isApplying,
    modifiedCode
}: FooterProps) {
    const { settings } = useSettings();
    const isLightTheme = settings?.theme === 'light';

    return (
        <div className={`p-3 border-t flex items-center justify-between ${
            isLightTheme ? 'border-zinc-600 bg-zinc-700' : 'border-zinc-700 bg-zinc-900'
        }`}>
            <div className="text-[10px] text-zinc-500 flex items-center gap-2">
            </div>

            <button
                onClick={onApply}
                disabled={isApplying || !modifiedCode.trim()}
                className={`flex items-center gap-2 px-4 py-1.5 rounded text-xs font-medium transition-colors ${isApplying || !modifiedCode.trim()
                    ? isLightTheme
                        ? 'bg-zinc-600 text-zinc-400 cursor-not-allowed'
                        : 'bg-zinc-700 text-zinc-500 cursor-not-allowed'
                    : 'bg-blue-600 hover:bg-blue-500 text-white shadow-lg shadow-blue-500/10'
                    }`}
                id="tour-apply"
            >
                {isApplying ? (
                    <>Применяю...</>
                ) : (
                    <>
                        <Check className="w-3.5 h-3.5" />
                        Применить
                    </>
                )}
            </button>
        </div>
    );
}
