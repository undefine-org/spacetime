// Product Card Web Component
class ProductCard extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({ mode: 'open' });
    }

    connectedCallback() {
        this.shadowRoot.innerHTML = `
            <style>
                :host {
                    display: block;
                    background: white;
                    border-radius: 12px;
                    overflow: hidden;
                    box-shadow: 0 2px 8px rgba(0,0,0,0.08);
                    transition: transform 0.2s ease, box-shadow 0.2s ease;
                }

                :host(:hover) {
                    transform: translateY(-4px);
                    box-shadow: 0 8px 24px rgba(0,0,0,0.12);
                }

                .image-container {
                    aspect-ratio: 4/3;
                    overflow: hidden;
                    background: #f0f0f0;
                }

                ::slotted([slot="image"]) {
                    width: 100%;
                    height: 100%;
                    object-fit: cover;
                }

                .content {
                    padding: 1.25rem;
                }

                .name {
                    font-size: 1.1rem;
                    font-weight: 600;
                    color: #333;
                    margin-bottom: 0.5rem;
                }

                .description {
                    font-size: 0.875rem;
                    color: #666;
                    margin-bottom: 1rem;
                    line-height: 1.4;
                }

                .footer {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                }

                .price {
                    font-size: 1.25rem;
                    font-weight: bold;
                    color: #0d9488;
                }

                .price::before {
                    content: "$";
                }

                button {
                    background: #0d9488;
                    color: white;
                    border: none;
                    padding: 0.625rem 1rem;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 0.875rem;
                    font-weight: 500;
                    transition: background 0.2s ease;
                }

                button:hover {
                    background: #0f766e;
                }

                button:active {
                    transform: scale(0.98);
                }
            </style>

            <div class="image-container">
                <slot name="image"></slot>
            </div>
            <div class="content">
                <div class="name"><slot name="name"></slot></div>
                <div class="description"><slot name="description"></slot></div>
                <div class="footer">
                    <span class="price"><slot name="price"></slot></span>
                    <button data-add-to-cart>Add to Cart</button>
                </div>
            </div>
        `;
    }
}

customElements.define('product-card', ProductCard);
