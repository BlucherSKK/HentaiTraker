import { encryptJson, decryptJson, tokenHash } from './crypto';

export interface VtnsTokens {
    pub_vtns: string;
    priv_vtns: string;
}

type WsState =
| { kind: 'long_token';    pub_vtns: string; priv_vtns: string }
| { kind: 'authenticated'; pub_at: string;   priv_at: string   }
| { kind: 'private_only';  priv_at: string                     }
| { kind: 'closed' };

type EventHandler = (event: string, payload: Record<string, unknown>) => void;

export class HntWsConnection {
    private ws: WebSocket;
    private state: WsState;
    private handlers = new Map<string, EventHandler[]>();

    constructor(ws: WebSocket, tokens: VtnsTokens) {
        this.ws    = ws;
        this.state = { kind: 'long_token', pub_vtns: tokens.pub_vtns, priv_vtns: tokens.priv_vtns };

        this.ws.binaryType = 'arraybuffer';
        this.ws.addEventListener('message', (e) => this.onMessage(e).catch(console.error));
        this.ws.addEventListener('close',   () => { this.state = { kind: 'closed' }; });
    }

    on(event: string, handler: EventHandler): this {
        if (!this.handlers.has(event)) this.handlers.set(event, []);
        this.handlers.get(event)!.push(handler);
        return this;
    }

    once(event: string, handler: EventHandler): () => void {
        const wrapper: EventHandler = (ev, payload) => {
            this.off(event, wrapper);
            handler(ev, payload);
        };
        this.on(event, wrapper);
        return () => this.off(event, wrapper);
    }

    off(event: string, handler: EventHandler): this {
        const list = this.handlers.get(event);
        if (list) this.handlers.set(event, list.filter(h => h !== handler));
        return this;
    }



    private emit(event: string, payload: Record<string, unknown>) {
        this.handlers.get(event)?.forEach(h => h(event, payload));
        this.handlers.get('*')?.forEach(h => h(event, payload));
    }

    private async onMessage(e: MessageEvent) {
        if (!(e.data instanceof ArrayBuffer)) {
            this.handleText(e.data as string);
            return;
        }
        await this.handleBinary(new Uint8Array(e.data));
    }

    private handleText(raw: string) {
        let obj: Record<string, unknown>;
        try { obj = JSON.parse(raw); } catch { return; }

        const event = obj.event as string;
        if (event === 'reauth_required') {
            this.doReauth().catch(console.error);
        } else {
            this.emit(event, obj);
        }
    }

    private async handleBinary(data: Uint8Array) {
        const s = this.state;

        if (s.kind === 'long_token') {
            const obj = await decryptJson(s.priv_vtns, data);
            if (!obj) return;
            const event = obj.event as string;

            if (event === 'login_ok' || event === 'register_ok') {
                this.state = {
                    kind:    'authenticated',
                    pub_at:  obj.pub_at  as string,
                    priv_at: obj.priv_at as string,
                };
                this.emit(event, obj);
            } else if (event === 'token_refresh') {
                this.state = {
                    kind:      'long_token',
                    pub_vtns:  obj.public_vtns  as string,
                    priv_vtns: obj.private_vtns as string,
                };
            }

        } else if (s.kind === 'authenticated') {
            const obj = await decryptJson(s.priv_at, data);
            if (!obj) {
                this.state = { kind: 'private_only', priv_at: s.priv_at };
                return;
            }
            const event = obj.event as string;

            if (event === 'token_refresh') {
                this.state = {
                    kind:    'authenticated',
                    pub_at:  obj.pub_at  as string,
                    priv_at: obj.priv_at as string,
                };
                return;
            }
            if (event === 'reauth_ok') {
                this.state = { kind: 'authenticated', pub_at: obj.pub_at as string, priv_at: s.priv_at };
                this.emit('reauth_ok', obj);
                return;
            }
            this.emit(event, obj);

        } else if (s.kind === 'private_only') {
            const obj = await decryptJson(s.priv_at, data);
            if (obj) this.emit(obj.event as string, obj);
        }
    }

    async login(username: string, password: string): Promise<void> {
        if (this.state.kind !== 'long_token') throw new Error('WS not in long_token state');
        const enc = await encryptJson(this.state.priv_vtns, { event: 'login', username, password });
        this.ws.send(enc);
    }

    async register(username: string, password: string, inviteToken: string): Promise<void> {
        if (this.state.kind !== 'long_token') throw new Error('WS not in long_token state');
        const enc = await encryptJson(this.state.priv_vtns, {
            event:        'register',
            username,
            password,
            invite_token: inviteToken,
        });
        this.ws.send(enc);
    }

    async send(event: string, payload: Record<string, unknown> = {}): Promise<void> {
        if (this.state.kind !== 'authenticated') throw new Error('WS not authenticated');
        const enc = await encryptJson(this.state.priv_at, { event, ...payload });
        this.ws.send(enc);
    }

    private async doReauth(): Promise<void> {
        if (this.state.kind !== 'authenticated' && this.state.kind !== 'private_only') return;
        const priv_at = (this.state as { priv_at: string }).priv_at;
        const hash    = await tokenHash(priv_at);
        this.state    = { kind: 'private_only', priv_at };
        this.ws.send(JSON.stringify({ event: 'reauth', hash }));
    }

    get isAuthenticated(): boolean { return this.state.kind === 'authenticated'; }
    get isConnected():     boolean { return this.ws.readyState === WebSocket.OPEN; }

    close() { this.ws.close(); }
}
