/// Data Primitives Index
///
/// This module provides composable primitives for reactive data binding.
///
/// ## Data Sources (source.st)
///
/// - `data-source(name, src, default?, refresh?)` - Fetch data from URL/localStorage
/// - `persist(key, data, debounce?)` - Save data to localStorage
/// - `computed(source, compute, deps?)` - Derived/computed values
///
/// ## Template Iteration (each.st)
///
/// - `each(&container, source, template, key?)` - Render template for each item
/// - `keyed-each(&container, source, template, key?)` - Optimized with key-based diffing
///
/// ## Data Bindings (binding.st)
///
/// - `bind-text(&el, value, format?)` - Bind data to text content
/// - `bind-attr(&el, attr, value)` - Bind data to attribute
/// - `bind-class(&el, className, condition)` - Conditional class toggle
/// - `bind-style(&el, property, value)` - Bind data to CSS property
/// - `bind-input(&el, value, event?, debounce?)` - Two-way input binding
/// - `bind-visible(&el, condition, mode?)` - Show/hide based on condition
/// - `bind-if(&el, condition)` - Conditional DOM insertion
///
/// ## Filters (filters.st)
///
/// Array operations:
/// - `filter(data, predicate, value?)` - Filter array by predicate
/// - `map(data, transform)` - Transform array items
/// - `sort(data, by?, order?)` - Sort array
/// - `slice(data, start?, end?)` - Take portion of array
/// - `take(data, count?, from?)` - First/last N items
/// - `reduce(data, reducer, initial?)` - Reduce to single value
/// - `unique(data, by?)` - Remove duplicates
/// - `group-by(data, by)` - Group items by property
///
/// Formatters:
/// - `uppercase(data)` - Convert to uppercase
/// - `lowercase(data)` - Convert to lowercase
/// - `capitalize(data)` - Capitalize first letter
/// - `truncate(data, length?, suffix?)` - Truncate with ellipsis
/// - `currency(data, locale?, currency?)` - Format as currency
/// - `number-format(data, locale?, decimals?)` - Format number
/// - `date-format(data, format?, locale?)` - Format date
///
/// ## Usage
///
/// Primitives are typically composed via macros:
///
/// ```st
/// .product-list {
///   @data(src: "/api/products") {
///     @each(template: "#product-card") {
///       .name { text: $.name }
///       .price { text: $.price | currency }
///     }
///   }
/// }
/// ```

@import "./event-source.st"
@import "./vendor.st"

