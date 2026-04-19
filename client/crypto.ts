/**
 * Превращает пароль в ключ для AES-256 GCM через PBKDF2
 */
export async function getEncryptionKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
    const enc = new TextEncoder();

    // 1. Импортируем "сырой" пароль
    const keyMaterial = await window.crypto.subtle.importKey(
        "raw",
        enc.encode(password),
        "PBKDF2",
        false,
        ["deriveKey"]
    );

    // 2. Растягиваем его до 256-битного ключа (хеширование)
    return window.crypto.subtle.deriveKey(
        {
            name: "PBKDF2",
            salt: salt,
            iterations: 100000,
            hash: "SHA-256",
        },
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
    const salt = window.crypto.getRandomValues(new Uint8Array(16)); // Случайная соль
    const iv = window.crypto.getRandomValues(new Uint8Array(12));   // Initialization Vector для GCM

    // Шифруем данные
    const encryptedContent = await window.crypto.subtle.encrypt(
        { name: "AES-GCM", iv: iv },
        token,
        enc.encode(JSON.stringify(payload))
    );

    // Формируем JWE-подобную структуру (Base64Url)
    const toBase64 = (buf: Uint8Array | ArrayBuffer) =>
    btoa(String.fromCharCode(...new Uint8Array(buf)))
    .replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');

    const header = toBase64(enc.encode(JSON.stringify({ alg: "dir", enc: "A256GCM" })));
    const encodedIv = toBase64(iv);
    const encodedCiphertext = toBase64(encryptedContent);
    const encodedSalt = toBase64(salt);

    // JWE Compact Serialization: header.encrypted_key.iv.ciphertext.tag
    // Так как у нас 'dir', encrypted_key пустой.
    // Мы также добавим соль в начало или конец для возможности дешифровки.
    return `${header}..${encodedIv}.${encodedCiphertext}.${encodedSalt}`;
}
