import type { McpServerConfig } from '../components/settings/MCPSettings';

export const BUILTIN_1C_SEARCH_ID = 'builtin-1c-search';
export const SEARCH_PROFILES_ENV = 'ONEC_CONFIG_PROFILES_JSON';
export const SEARCH_ACTIVE_PROFILE_ENV = 'ONEC_CONFIG_ACTIVE_PROFILE_ID';

export interface SearchExtensionProfile {
    id: string;
    name: string;
    path: string;
}

export interface SearchConfigProfile {
    id: string;
    name: string;
    main_path: string;
    extensions: SearchExtensionProfile[];
}

export function normalizeSearchProfiles(
    server: McpServerConfig,
): { profiles: SearchConfigProfile[]; activeId: string } {
    const env = server.env || {};
    const legacyPath = env['ONEC_CONFIG_PATH'] || '';
    let profiles: SearchConfigProfile[] = [];

    try {
        const parsed = JSON.parse(env[SEARCH_PROFILES_ENV] || '[]');
        if (Array.isArray(parsed)) {
            profiles = parsed
                .filter(p => p && typeof p === 'object')
                .map((p, idx) => ({
                    id: String(p.id || `profile-${idx + 1}`),
                    name: typeof p.name === 'string' ? p.name : `Конфигурация ${idx + 1}`,
                    main_path: String(p.main_path || ''),
                    extensions: Array.isArray(p.extensions)
                        ? p.extensions.map((e: any, extIdx: number) => ({
                            id: String(e.id || `ext-${extIdx + 1}`),
                            name: typeof e.name === 'string' ? e.name : `Расширение ${extIdx + 1}`,
                            path: String(e.path || ''),
                        }))
                        : [],
                }));
        }
    } catch {
        profiles = [];
    }

    if (profiles.length === 0) {
        const id = 'default';
        profiles = [{
            id,
            name: 'Основная конфигурация',
            main_path: legacyPath,
            extensions: [],
        }];
    }

    const activeFromEnv = env[SEARCH_ACTIVE_PROFILE_ENV];
    const activeId = profiles.some(p => p.id === activeFromEnv)
        ? String(activeFromEnv)
        : profiles[0].id;

    return { profiles, activeId };
}

export function buildSearchEnv(
    server: McpServerConfig,
    profiles: SearchConfigProfile[],
    activeId: string,
): Record<string, string> {
    const activeProfile = profiles.find(p => p.id === activeId) || profiles[0];
    return {
        ...(server.env || {}),
        ONEC_CONFIG_PATH: activeProfile?.main_path || '',
        [SEARCH_ACTIVE_PROFILE_ENV]: activeProfile?.id || activeId,
        [SEARCH_PROFILES_ENV]: JSON.stringify(profiles),
    };
}
