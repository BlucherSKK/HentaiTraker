


export class HomeNav extends HTMLElement {



    static get observedAttributes() {
        return ['data-value'];
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
        const val = this.getAttribute('data-value') || 'Пусто';
        this.innerHTML = `<div>Значение: ${val}</div>`;
    }
}

export function define_homenav(){
        customElements.define('home-nav', HomeNav);
}
