import { invoke } from '@tauri-apps/api/core';

/**
 * Launch 1C Configurator for a given infobase.
 * @param platformPath - path to 1cv8.exe
 * @param infobasePath - connection string or path
 * @param isServer - true for server bases, false for file bases
 * @param login - optional 1C user login
 * @param password - optional 1C user password
 */
export async function launchConfigurator(
    platformPath: string,
    infobasePath: string,
    isServer: boolean,
    login?: string,
    password?: string,
): Promise<void> {
    return await invoke('launch_configurator_cmd', { platformPath, infobasePath, isServer, login: login ?? null, password: password ?? null });
}

/**
 * Get list of 1C infobases from builtin-jvv-1c MCP server.
 * Returns array of { name, connection, type, id, folder }
 */
export async function get1cInfobases(): Promise<Array<{
    name: string;
    connection: string;
    type: 'file' | 'server';
    id: string | null;
    folder: string | null;
}>> {
    return await invoke('get_1c_infobases_cmd');
}

/**
 * Get path to the latest installed 1C platform (1cv8.exe).
 */
export async function get1cPlatformPath(): Promise<string> {
    return await invoke('get_1c_platform_path_cmd');
}
