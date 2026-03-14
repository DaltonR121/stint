import * as vscode from 'vscode';
import { ChildProcess, spawn } from 'child_process';

let statusBarItem: vscode.StatusBarItem;
let intervalId: ReturnType<typeof setInterval> | undefined;
let serverProcess: ChildProcess | undefined;
let startAttempted = false;

interface StatusResponse {
    tracking: boolean;
    project: string | null;
    elapsed_secs: number | null;
}

export function activate(context: vscode.ExtensionContext): void {
    statusBarItem = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Left,
        100
    );
    statusBarItem.show();
    context.subscriptions.push(statusBarItem);

    // Poll immediately, then on interval
    updateStatus();
    const config = vscode.workspace.getConfiguration('stint');
    const pollInterval = config.get<number>('pollInterval', 3000);
    intervalId = setInterval(updateStatus, pollInterval);

    // Re-configure on settings change
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration((e) => {
            if (e.affectsConfiguration('stint')) {
                if (intervalId) {
                    clearInterval(intervalId);
                }
                const newConfig = vscode.workspace.getConfiguration('stint');
                const newInterval = newConfig.get<number>('pollInterval', 3000);
                intervalId = setInterval(updateStatus, newInterval);
            }
        })
    );
}

/**
 * Attempts to start `stint serve` as a background child process.
 * Only tries once per session to avoid spawn loops.
 */
function tryStartServer(): void {
    if (startAttempted || serverProcess) {
        return;
    }
    startAttempted = true;

    const config = vscode.workspace.getConfiguration('stint');
    const stintPath = config.get<string>('stintPath', 'stint');

    try {
        let port = '7653';
        try {
            const url = new URL(config.get<string>('apiUrl', 'http://127.0.0.1:7653'));
            if (url.port) {
                port = url.port;
            }
        } catch {
            // Invalid URL in config — use default port
        }

        serverProcess = spawn(stintPath, ['serve', '--port', port], {
            detached: true,
            stdio: 'ignore',
        });

        // Detach so the server outlives VS Code if needed
        serverProcess.unref();

        serverProcess.on('error', () => {
            serverProcess = undefined;
        });

        serverProcess.on('exit', () => {
            serverProcess = undefined;
        });
    } catch {
        // stint not found or spawn failed — silently ignore
    }
}

async function updateStatus(): Promise<void> {
    const config = vscode.workspace.getConfiguration('stint');
    const apiUrl = config.get<string>('apiUrl', 'http://127.0.0.1:7653');

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 3000);
    try {
        const response = await fetch(`${apiUrl}/api/status`, {
            signal: controller.signal,
        });
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }

        const data = (await response.json()) as StatusResponse;

        if (data.tracking && data.project && data.elapsed_secs !== null) {
            const elapsed = formatDuration(data.elapsed_secs);
            statusBarItem.text = `$(clock) ${data.project} ${elapsed}`;
            statusBarItem.tooltip = `Stint: tracking ${data.project}`;
            statusBarItem.color = new vscode.ThemeColor('charts.green');
        } else {
            statusBarItem.text = `$(clock) idle`;
            statusBarItem.tooltip = 'Stint: no timer running';
            statusBarItem.color = undefined;
        }
    } catch {
        // API not reachable — try to start the server
        tryStartServer();
        statusBarItem.text = `$(clock) stint starting...`;
        statusBarItem.tooltip = 'Stint: starting API server...';
        statusBarItem.color = undefined;
    } finally {
        clearTimeout(timeout);
    }
}

function formatDuration(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    if (h > 0 && m > 0) {
        return `${h}h ${m}m`;
    }
    if (h > 0) {
        return `${h}h`;
    }
    if (m > 0) {
        return `${m}m`;
    }
    return `${secs}s`;
}

export function deactivate(): void {
    if (intervalId) {
        clearInterval(intervalId);
    }
    // Don't kill the server — it may be serving other clients
}
