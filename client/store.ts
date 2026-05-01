export type ToastPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
export type Lang = 'ru' | 'en';
interface UserSettings {
    toast_position: ToastPosition;
}

const DEFAULTS: UserSettings = {
    toast_position: 'top-right',
};

interface Store {
    settings: UserSettings;
    lang: Lang;
}



let _store: Store = { settings: DEFAULTS, lang: 'en'};

export function getSettings(): UserSettings {
    return _store.settings;
}

export function getLang(): Lang {
    return _store.lang;
}

export function applySettings(raw: string | null | undefined): void {
    if (!raw) { _store = _store; return; }
    try {
        let settings: UserSettings = { ...DEFAULTS, ...JSON.parse(raw) };
        _store = {settings: settings, lang: _store.lang};
    } catch {
        _store = _store;
    }
}

export function updateSettings(patch: Partial<UserSettings>): void {
    let settings: UserSettings = { ...DEFAULTS, ...patch };
    _store = {settings: settings, lang: _store.lang};
}
