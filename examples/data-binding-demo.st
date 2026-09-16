// Spacetime Data Binding System Demo
// This file demonstrates the Phase 2 implementation

// Type definitions
@type Product {
    id: string;
    name: string;
    price: number;
    inStock: boolean;
    category: string;
    imageUrl?: url;
}

@type Category {
    id: string;
    name: string;
    products: Product[];
}

// Data sources (unified @data <kind> surface, PLAN-023)
@data fetch $products Product[] : "/api/products" {
    refresh: 5m;
}

@data fetch $categories Category[] : "/api/categories" {
    refresh: 10m;
}

// Computed data: array-query pipeline
@data query $activeProducts Product[] from $products {
    where: $.inStock == true;
    sort: $.price;
    dir: asc;
}

@data query $featuredProducts Product[] from $activeProducts {
    limit: 6;
}

// Helper functions
@fn formatPrice($price number): string {
    return "$" + price.toFixed(2);
}

@fn truncate($text string, $maxLength number): string {
    if (text.length > maxLength) {
        return text.substring(0, maxLength) + "...";
    }
    return text;
}

// Template definitions
@template &product-card($p) {
    <product-card data-id="`$p.id`" data-category="`$p.category`">
        <img slot="image" src="`$p.imageUrl`" alt="`$p.name`">
        <span slot="name">`$p.name`</span>
        <span slot="price">formatPrice(`$p.price`)</span>
        <span slot="category">`$p.category`</span>
        <span slot="discount">formatPrice(`$p.price` * 0.9)</span>
    </product-card>
}

@template &product-item($item) {
    <product-item>
        <span slot="name">`$item.name` | truncate(50)</span>
        <span slot="price">`$item.price` | formatPrice</span>
    </product-item>
}

@template &category-section($cat) {
    <category-section>
        <span slot="title">`$cat.name`</span>
        <div slot="products"></div>
    </category-section>
}

// Data binding with @each
.product-grid {
    @each($featuredProducts as $p) {
        &product-card($p);
    }
}

.category-list {
    @each($categories as $cat) {
        &category-section($cat);

        // Nested iteration for products within each category
        > category-section [slot="products"] {
            @each($cat.products as $item) {
                &product-item($item);
            }
        }
    }
}

// Animations still work alongside data binding
.product-grid {
    @on visible entrance {
        opacity: 0 -> 1;
        translate-y: 20px -> 0;
    }
}
