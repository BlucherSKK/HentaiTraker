
export class AppNav extends HTMLElement {

    static get observedAttributes() {
        return ['data-link', 'data-user-roles'];
    }

    attributeChangedCallback(name: string, oldValue: string, newValue: string) {
        if (oldValue !== newValue) this.render();
    }

    connectedCallback() { this.render(); }

    render() {
        const rolesStr = this.getAttribute('data-user-roles') || '';
        const roles    = rolesStr.split(',').map(r => r.trim()).filter(Boolean);
        const isAdmin  = roles.includes('admin');
        const canPost  = roles.some(r => r === 'admin' || r === 'force_posting');
        const page     = this.getAttribute('data-link') || 'feeds';

        const btn = (link: string, label: string) =>
        `<button class="${page === link ? 'nav-btn-here' : 'nav-btn'}" data-link="${link}">${label}</button>`;

        this.innerHTML = `
        <div class="nav-container">
        <div class="nav-left">
        ${btn('dm',    'личка')}
        ${btn('chats', 'чаты')}
        ${btn('feeds', 'лента')}
        ${canPost ? btn('post-create', '+ пост') : ''}
        </div>
        ${isAdmin ? `<div class="nav-right">${btn('terminal', 'терминал')}</div>` : ''}
        </div>`;
    }
}
