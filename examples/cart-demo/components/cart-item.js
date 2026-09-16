// Cart Item Web Component
class CartItem extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({ mode: 'open' });
    }

    connectedCallback() {
        this.shadowRoot.innerHTML = `
            <style>
                :host {
                    display: flex;
                    gap: 1rem;
                    padding: 1rem 0;
                    border-bottom: 1px solid #eee;
                }

                :host(:last-child) {
                    border-bottom: none;
                }

                .image-container {
                    width: 80px;
                    height: 80px;
                    border-radius: 8px;
                    overflow: hidden;
                    flex-shrink: 0;
                    background: #f0f0f0;
                }

                ::slotted([slot="image"]) {
                    width: 100%;
                    height: 100%;
                    object-fit: cover;
                }

                .details {
                    flex: 1;
                    display: flex;
                    flex-direction: column;
                    justify-content: space-between;
                }

                .name {
                    font-weight: 600;
                    color: #333;
                    font-size: 0.95rem;
                }

                .price {
                    color: #0d9488;
                    font-weight: 500;
                }

                .price::before {
                    content: "$";
                }

                .controls {
                    display: flex;
                    align-items: center;
                    gap: 0.5rem;
                }

                .qty-control {
                    display: flex;
                    align-items: center;
                    gap: 0.5rem;
                    background: #f5f5f5;
                    border-radius: 6px;
                    padding: 0.25rem;
                }

                .qty-btn {
                    width: 28px;
                    height: 28px;
                    border: none;
                    background: white;
                    border-radius: 4px;
                    cursor: pointer;
                    font-size: 1rem;
                    color: #666;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                }

                .qty-btn:hover {
                    background: #e5e5e5;
                }

                .qty {
                    min-width: 2rem;
                    text-align: center;
                    font-weight: 500;
                }

                .remove-btn {
                    background: none;
                    border: none;
                    color: #dc2626;
                    cursor: pointer;
                    padding: 0.5rem;
                    font-size: 0.875rem;
                    opacity: 0.7;
                    transition: opacity 0.2s;
                }

                .remove-btn:hover {
                    opacity: 1;
                }
            </style>

            <div class="image-container">
                <slot name="image"></slot>
            </div>
            <div class="details">
                <div>
                    <div class="name"><slot name="name"></slot></div>
                    <div class="price"><slot name="price"></slot></div>
                </div>
                <div class="controls">
                    <div class="qty-control">
                        <button class="qty-btn" data-decrement>−</button>
                        <span class="qty"><slot name="qty"></slot></span>
                        <button class="qty-btn" data-increment>+</button>
                    </div>
                    <button class="remove-btn" data-remove>Remove</button>
                </div>
            </div>
        `;
    }
}

customElements.define('cart-item', CartItem);
