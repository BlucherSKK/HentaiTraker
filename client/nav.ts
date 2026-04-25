


export class AppNav extends HTMLElement {

    static get observedAttributes() {
        return ['data-link'];
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

        const roles = this.getAttribute('data-user-roles') || '';
        const isAdmin = roles.split(',').map(r => r.trim()).includes('admin');

        const page = this.getAttribute('data-link') || 'feeds';
        this.innerHTML = `
        <div class="nav-container">
            <button class=${page === "dm" ? "nav-btn-here" : "nav-btn"} data-link="dm">личка</button>
            <button class=${page === "chats" ? "nav-btn-here" : "nav-btn"} data-link="chats">чаты</button>
            <button class=${page === "feeds" ? "nav-btn-here" : "nav-btn"} data-link="feeds">ленты</button>
            ${ isAdmin ? `<button class=${page === 'terminal' ? 'nav-btn-here' : 'nav-btn'} data-link='terminal'>терминал</buttin>` : ""}
        </div>   `;
    }
}
