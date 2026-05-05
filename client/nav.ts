export class AppNav extends HTMLElement {

    static get observedAttributes() {
        return ['data-link', 'data-user-roles', 'data-user-id'];
    }

    attributeChangedCallback(name: string, oldValue: string, newValue: string) {
        if (oldValue !== newValue) this.render();
    }

    connectedCallback() {
        this.render();
    }

    private _bindRefresh() {
        this.querySelector('#nav-refresh-btn')?.addEventListener('click', () => {
            window.dispatchEvent(new CustomEvent('feed-refresh'));
        });
    }

    render() {
        const rolesStr = this.getAttribute('data-user-roles') || '';
        const roles    = rolesStr.split(',').map(r => r.trim()).filter(Boolean);
        const isAdmin  = roles.includes('admin');
        const canPost  = roles.some(r => r === 'admin' || r === 'force_posting');
        const isAuth   = !!this.getAttribute('data-user-id');
        const page     = this.getAttribute('data-link') || 'feeds';

        const btn = (link: string, label: string) =>
        `<button class="${page === link ? 'f-btn-selected' : 'f-btn'}" data-link="${link}">${label}</button>`;

        this.innerHTML = `
        <div class="f-tab-btns">
            <div class='left-pane'>
                ${btn('dm',    'личка')}
                ${btn('chats', 'чаты')}
                ${btn('feeds', 'лента')}
                ${canPost && page === 'feeds' ? btn('post-create', '+ пост') : ''}
                ${page === 'feeds' ? `<button class="f-btn nav-refresh-btn" id="nav-refresh-btn" title="Обновить ленту">↻</button>` : ''}
            </div>
            <div class="right-pane">
                ${isAuth ? btn('settings', 'настройки') : ''}
                ${isAdmin ? btn('terminal', 'терминал') : ''}
            </div>
        </div>`;

        this._bindRefresh();
     }
}
