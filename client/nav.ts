



export class AppNav extends HTMLElement {

    static get observedAttributes() {
        return ['data-link', 'data-user-roles'];
    }

    attributeChangedCallback(name: string, oldValue: string, newValue: string) {
        if (oldValue !== newValue) {
            this.render();
        }
    }

    connectedCallback() {
        this.render();
    }

    render() {
        const rolesStr = this.getAttribute('data-user-roles') || '';
        const roles    = rolesStr.split(',').map(r => r.trim()).filter(Boolean);
        const isAdmin   = roles.includes('admin');
        const canPost   = roles.some(r =>
        r === 'admin' || r === 'force_posting'
        );

        const page = this.getAttribute('data-link') || 'feeds';
        this.innerHTML = `
        <div class="nav-container">
        <button class=${page === "dm"    ? "nav-btn-here" : "nav-btn"} data-link="dm">личка</button>
        <button class=${page === "chats" ? "nav-btn-here" : "nav-btn"} data-link="chats">чаты</button>
        <button class=${page === "feeds" ? "nav-btn-here" : "nav-btn"} data-link="feeds">лента</button>
        ${canPost  ? `<button class=${page === 'post-create' ? 'nav-btn-here' : 'nav-btn'} data-link='post-create'>+ пост</button>` : ''}
        ${isAdmin  ? `<button class=${page === 'terminal'    ? 'nav-btn-here' : 'nav-btn'} data-link='terminal'>терминал</button>` : ''}
        </div>`;
    }
}
