export type ToastPosition = 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
export type ToastSound    = 'none' | 'soft' | 'sharp';
export type Lang          = 'ru' | 'en';

export interface UserSettings {
    toast_position:     ToastPosition;
    toast_sound_info:   ToastSound;
    toast_sound_success: ToastSound;
    toast_sound_warn:   ToastSound;
    toast_sound_error:  ToastSound;
}

const DEFAULTS: UserSettings = {
    toast_position:      'top-right',
    toast_sound_info:    'soft',
    toast_sound_success: 'soft',
    toast_sound_warn:    'soft',
    toast_sound_error:   'sharp',
};

interface Store {
    settings: UserSettings;
    lang:     Lang;
}

let _store: Store = { settings: { ...DEFAULTS }, lang: 'en' };

export function getSettings(): UserSettings {
    return _store.settings;
}

export function getLang(): Lang {
    return _store.lang;
}

export function applySettings(raw: string | null | undefined): void {
    if (!raw) return;
    try {
        const parsed: UserSettings = { ...DEFAULTS, ...JSON.parse(raw) };
        _store = { settings: parsed, lang: _store.lang };
    } catch {
        /* keep current */
    }
}

export function updateSettings(patch: Partial<UserSettings>): void {
    _store = { settings: { ..._store.settings, ...patch }, lang: _store.lang };
}
