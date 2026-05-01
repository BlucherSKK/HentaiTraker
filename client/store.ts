export type ToastPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export interface UserSettings {
    toast_position: ToastPosition;
}

// ----- defaults -----

const DEFAULTS: UserSettings = {
    toast_position: 'top-right',
};

// ----- store -----

let _store: UserSettings = { ...DEFAULTS };

export function getSettings(): UserSettings {
    return _store;
}

export function applySettings(raw: string | null | undefined): void {
    if (!raw) { _store = { ...DEFAULTS }; return; }
    try {
        _store = { ...DEFAULTS, ...JSON.parse(raw) };
    } catch {
        _store = { ...DEFAULTS };
    }
}

export function updateSettings(patch: Partial<UserSettings>): void {
    _store = { ..._store, ...patch };
}
