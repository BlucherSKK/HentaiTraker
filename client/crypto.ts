/**
 * Превращает пароль в ключ для AES-256 GCM через PBKDF2
 */
export async function getEncryptionKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
    const enc = new TextEncoder();
    const keyMaterial = await window.crypto.subtle.importKey(
        "raw",
        enc.encode(password),
                                                             "PBKDF2",
                                                             false,
                                                             ["deriveKey"]
    );
    return window.crypto.subtle.deriveKey(
        { name: "PBKDF2", salt, iterations: 100000, hash: "SHA-256" },
        keyMaterial,
        { name: "AES-GCM", length: 256 },
        false,
        ["encrypt"]
    );
}

/**
 * Шифрует JSON в формат, близкий к JWE (Compact Serialization)
 */
export async function encryptJsonToJwe(payload: object, token: CryptoKey): Promise<string> {
    const enc = new TextEncoder();
    const salt = window.crypto.getRandomValues(new Uint8Array(16));
    const iv   = window.crypto.getRandomValues(new Uint8Array(12));

    const encryptedContent = await window.crypto.subtle.encrypt(
        { name: "AES-GCM", iv },
        token,
        enc.encode(JSON.stringify(payload))
    );

    const toBase64 = (buf: Uint8Array | ArrayBuffer) =>
    btoa(String.fromCharCode(...new Uint8Array(buf)))
    .replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

    const header           = toBase64(enc.encode(JSON.stringify({ alg: "dir", enc: "A256GCM" })));
    const encodedIv        = toBase64(iv);
    const encodedCiphertext = toBase64(encryptedContent);
    const encodedSalt      = toBase64(salt);

    return `${header}..${encodedIv}.${encodedCiphertext}.${encodedSalt}`;
}

// ─── Серверно-совместимые функции ────────────────────────────────────────────
//
// Формат шифрования совпадает с src/secure.rs:
//   key  = SHA-256(keyMaterial)
//   wire = nonce(12 байт) || ciphertext+tag

async function deriveAesKey(keyMaterial: string, usage: KeyUsage[]): Promise<CryptoKey> {
    const keyBytes = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(keyMaterial));
    return crypto.subtle.importKey('raw', keyBytes, { name: 'AES-GCM', length: 256 }, false, usage);
}

/** Шифрует binary-буфер ключом keyMaterial. Возвращает nonce(12) || ciphertext+tag */
export async function encryptBinary(keyMaterial: string, plaintext: Uint8Array): Promise<Uint8Array> {
    const key   = await deriveAesKey(keyMaterial, ['encrypt']);
    const nonce = crypto.getRandomValues(new Uint8Array(12));
    const ct    = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: nonce }, key, plaintext);
    const out   = new Uint8Array(12 + ct.byteLength);
    out.set(nonce, 0);
    out.set(new Uint8Array(ct), 12);
    return out;
}

/** Дешифрует буфер формата nonce(12) || ciphertext+tag. Возвращает null при ошибке */
export async function decryptBinary(keyMaterial: string, data: Uint8Array): Promise<Uint8Array | null> {
    if (data.length < 12) return null;
    try {
        const key  = await deriveAesKey(keyMaterial, ['decrypt']);
        const plain = await crypto.subtle.decrypt(
            { name: 'AES-GCM', iv: data.slice(0, 12) },
                                                  key,
                                                  data.slice(12)
        );
        return new Uint8Array(plain);
    } catch {
        return null;
    }
}

/** Шифрует JSON-объект для отправки по WebSocket */
export async function encryptJson(keyMaterial: string, payload: object): Promise<Uint8Array> {
    return encryptBinary(keyMaterial, new TextEncoder().encode(JSON.stringify(payload)));
}

/** Дешифрует бинарное сообщение и парсит JSON. Возвращает null при любой ошибке */
export async function decryptJson(keyMaterial: string, data: Uint8Array): Promise<Record<string, unknown> | null> {
    const plain = await decryptBinary(keyMaterial, data);
    if (!plain) return null;
    try {
        return JSON.parse(new TextDecoder().decode(plain)) as Record<string, unknown>;
    } catch {
        return null;
    }
}

/** SHA-256 hex-дайджест строки токена — совпадает с secure::token_hash на сервере */
export async function tokenHash(token: string): Promise<string> {
    const bytes = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(token));
    return Array.from(new Uint8Array(bytes)).map(b => b.toString(16).padStart(2, '0')).join('');
}

/** Генерирует случайный токен как URL-safe base64 без padding — совпадает с secure::get_token */
export function generateToken(sizeInBytes: number): string {
    const bytes = crypto.getRandomValues(new Uint8Array(sizeInBytes));
    let bin = '';
    for (const b of bytes) bin += String.fromCharCode(b);
    return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
