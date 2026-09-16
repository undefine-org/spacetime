# End-to-End Test Plan for Spacetime Data Binding System

This document outlines the E2E testing strategy for the Spacetime data binding system. These tests verify the complete user experience from compilation through runtime execution in a browser.

---

## Table of Contents

1. [Test Environment Setup](#1-test-environment-setup)
2. [Core Functionality Tests](#2-core-functionality-tests)
3. [Data Loading Tests](#3-data-loading-tests)
4. [State Machine Integration Tests](#4-state-machine-integration-tests)
5. [Animation Integration Tests](#5-animation-integration-tests)
6. [Error Handling Tests](#6-error-handling-tests)
7. [Performance Tests](#7-performance-tests)
8. [Browser Compatibility Tests](#8-browser-compatibility-tests)

---

## 1. Test Environment Setup

### Prerequisites

- **Browser**: Chrome/Firefox/Safari (latest versions)
- **Test Server**: Local HTTP server for serving test pages
- **Test Data**: Mock JSON files in `/test-data/` directory
- **Automation**: Selenium/Playwright for automated browser testing (optional)

### Test Project Structure

```
tests/e2e/
├── fixtures/
│   ├── products.json
│   ├── prints.json
│   └── curations.json
├── pages/
│   ├── basic-binding.html
│   ├── state-machine.html
│   ├── animations.html
│   └── error-handling.html
└── specs/
    ├── basic-binding.spec.js
    ├── state-machine.spec.js
    └── animations.spec.js
```

---

## 2. Core Functionality Tests

### 2.1 Page Loads Without Console Errors

**Test ID**: E2E-001

**Objective**: Verify that pages with data bindings load without JavaScript errors.

**Steps**:
1. Compile a simple .st file with data bindings
2. Open the generated HTML in browser
3. Open browser DevTools console
4. Verify no errors are logged

**Expected Result**:
- Console shows no errors
- Page loads successfully
- No red error messages

**Example .st File**:
```css
@type Product {
    id: string;
    name: string;
}

@data products: Product[] {
    src: "/test-data/products.json";
}

.product-grid {
    @each(products) {
        template: "product-card";
        [slot="name"]: $.name;
    }
}
```

---

### 2.2 Data Fetches Successfully

**Test ID**: E2E-002

**Objective**: Verify that data is fetched from the specified URL.

**Steps**:
1. Set up a test JSON file at `/test-data/products.json`
2. Compile .st file referencing this data
3. Open page in browser
4. Check Network tab for successful fetch
5. Verify XHR/Fetch request returns 200 OK

**Expected Result**:
- Network request to `/test-data/products.json` succeeds
- Response contains valid JSON
- Response status is 200

**Verification**:
```javascript
// Browser console
window.__spacetime_data__.products !== undefined
window.__spacetime_data__.products.length > 0
```

---

### 2.3 All Items Render from JSON

**Test ID**: E2E-003

**Objective**: Verify that all items from the JSON file are rendered as DOM elements.

**Steps**:
1. Create JSON file with known number of items (e.g., 5 products)
2. Compile .st file with @each binding
3. Open page in browser
4. Wait for data to load
5. Count rendered elements

**Expected Result**:
- DOM contains exactly 5 product cards
- Each card has the correct data from JSON
- No items are missing or duplicated

**Verification**:
```javascript
// Browser console
const cards = document.querySelectorAll('product-card');
cards.length === 5; // Should be true

// Verify first card has correct data
const firstCard = cards[0];
firstCard.querySelector('[slot="name"]').textContent === "Product 1";
```

---

### 2.4 Slot Bindings Are Correct

**Test ID**: E2E-004

**Objective**: Verify that slot bindings correctly populate template elements.

**Steps**:
1. Define template with multiple slots (name, price, image)
2. Compile with bindings for each slot
3. Open page and inspect rendered elements

**Expected Result**:
- Text slots contain correct data
- Attribute bindings (src, alt) are set correctly
- Host bindings (data-* attributes) are present

**Example Verification**:
```javascript
const card = document.querySelector('product-card');
const image = card.querySelector('[slot="image"]');

// Text content
card.querySelector('[slot="name"]').textContent === "Product 1";

// Attributes
image.getAttribute('src') === "/images/product-1.jpg";
image.getAttribute('alt') === "Product 1";

// Host attributes
card.getAttribute('data-id') === "P001";
```

---

### 2.5 Nested Property Paths Work

**Test ID**: E2E-005

**Objective**: Verify that nested property access (e.g., `$.author.name`) works correctly.

**Steps**:
1. Create JSON with nested objects
2. Bind to nested properties
3. Verify rendered values

**Expected Result**:
- Nested properties are correctly accessed
- No undefined/null errors in console

**Example**:
```javascript
// JSON
{
    "title": "Post 1",
    "author": {
        "name": "John Doe",
        "email": "john@example.com"
    }
}

// Rendered
document.querySelector('[slot="author"]').textContent === "John Doe";
```

---

## 3. Data Loading Tests

### 3.1 URL Source Loading

**Test ID**: E2E-010

**Objective**: Verify data loads from a URL source.

**Steps**:
1. Set up JSON endpoint
2. Compile with `src: "/api/products"`
3. Monitor network requests
4. Verify data populates

**Expected Result**:
- Fetch request is made to the correct URL
- Data is parsed and rendered
- Loading completes successfully

---

### 3.2 localStorage Source Loading

**Test ID**: E2E-011

**Objective**: Verify data loads from localStorage.

**Steps**:
1. Pre-populate localStorage with test data
2. Compile with `src: localStorage("cart")`
3. Verify data renders from localStorage

**Expected Result**:
- Data is read from localStorage
- Items render correctly
- No fetch requests are made

**Verification**:
```javascript
// Setup
localStorage.setItem('cart', JSON.stringify([
    { productId: "P001", quantity: 2 }
]));

// After page load
document.querySelectorAll('cart-item').length === 1;
```

---

### 3.3 Default Values for Empty localStorage

**Test ID**: E2E-012

**Objective**: Verify that default values are used when localStorage is empty.

**Steps**:
1. Clear localStorage
2. Compile with `default: []`
3. Verify empty state is handled

**Expected Result**:
- No errors when localStorage is empty
- Default value is used
- Empty state triggers if configured

---

### 3.4 Computed Data Updates

**Test ID**: E2E-013

**Objective**: Verify that computed data derives correctly from source data.

**Steps**:
1. Define source data and computed filter
2. Verify computed data contains only filtered items

**Expected Result**:
- Computed data contains correct subset
- Filters are applied correctly
- Rendering reflects computed data, not source

---

## 4. State Machine Integration Tests

### 4.1 Loading State Appears First

**Test ID**: E2E-020

**Objective**: Verify that loading state is shown while data is fetching.

**Steps**:
1. Compile with state machine and loading state
2. Open page (with network throttling if needed)
3. Observe loading state appears first

**Expected Result**:
- Element has `data-state="loading"` attribute
- Loading styles are applied
- State is visible before data loads

**Verification**:
```javascript
// Immediately on page load
const container = document.querySelector('.gallery');
container.getAttribute('data-state') === 'loading';

// Loading styles should be visible
getComputedStyle(container).opacity === '0.5';
```

---

### 4.2 Ready State After Data Loads

**Test ID**: E2E-021

**Objective**: Verify transition to ready state after successful data load.

**Steps**:
1. Configure state machine with `data:products:loaded` transition
2. Load page
3. Wait for data to load
4. Verify state changes to "ready"

**Expected Result**:
- State changes from "loading" to "ready"
- Ready state styles are applied
- Data is rendered

**Verification**:
```javascript
// After data loads
await waitForDataLoad();
container.getAttribute('data-state') === 'ready';
getComputedStyle(container).opacity === '1';
```

---

### 4.3 Empty State When No Data

**Test ID**: E2E-022

**Objective**: Verify empty state when data array is empty.

**Steps**:
1. Return empty array from API
2. Configure empty state transition
3. Verify empty state is shown

**Expected Result**:
- `data:products:empty` event fires
- State transitions to "empty"
- Empty state message is shown

---

### 4.4 Error State on Network Failure

**Test ID**: E2E-023

**Objective**: Verify error state on failed data fetch.

**Steps**:
1. Configure invalid URL or return 404
2. Set up error state
3. Verify error state is shown

**Expected Result**:
- `data:products:error` event fires
- State transitions to "error"
- Error message is displayed

---

## 5. Animation Integration Tests

### 5.1 Scroll Animations Trigger Correctly

**Test ID**: E2E-030

**Objective**: Verify that generated elements participate in scroll animations.

**Steps**:
1. Compile with @scroll reveal animation
2. Render items
3. Scroll page to trigger animations
4. Verify animations execute

**Expected Result**:
- Generated elements have animation styles
- Animations trigger on scroll
- Stagger delays are applied correctly

**Example**:
```css
.product-grid {
    @each(products) {
        template: "product-card";
        [slot="name"]: $.name;
    }

    > product-card {
        @scroll reveal(&quick-reveal) {
            opacity: 0 -> 1;
            translate-y: 40px -> 0;
            stagger: 0.08 first;
        }
    }
}
```

**Verification**:
- Cards start with `opacity: 0`
- Scrolling into view triggers fade-in
- Each card animates with 80ms stagger

---

### 5.2 Hover Animations on Generated Elements

**Test ID**: E2E-031

**Objective**: Verify hover animations work on dynamically generated elements.

**Steps**:
1. Compile with @on hover animation
2. Render items
3. Hover over generated elements
4. Verify hover animation triggers

**Expected Result**:
- Hover animations apply to all generated items
- Animations are smooth and consistent
- No JavaScript errors

---

### 5.3 State-Based Animations

**Test ID**: E2E-032

**Objective**: Verify animations work during state transitions.

**Steps**:
1. Configure loading → ready transition with animation
2. Observe animation during state change

**Expected Result**:
- Transition animates smoothly
- CSS transitions are applied
- Final state is correct

---

## 6. Error Handling Tests

### 6.1 Network Timeout

**Test ID**: E2E-040

**Objective**: Verify graceful handling of network timeouts.

**Steps**:
1. Configure slow/timeout endpoint
2. Set reasonable timeout
3. Verify error state is shown

**Expected Result**:
- Timeout triggers error state
- User sees error message
- No JavaScript exceptions

---

### 6.2 Malformed JSON Response

**Test ID**: E2E-041

**Objective**: Verify handling of invalid JSON.

**Steps**:
1. Return malformed JSON from endpoint
2. Verify error state

**Expected Result**:
- JSON parse error is caught
- Error state is shown
- Console shows descriptive error

---

### 6.3 Missing Required Fields

**Test ID**: E2E-042

**Objective**: Verify handling of JSON with missing fields.

**Steps**:
1. Return JSON missing required fields
2. Verify error handling

**Expected Result**:
- Validation error is logged
- Items with errors are skipped or show default values
- Page doesn't crash

---

### 6.4 Type Mismatch

**Test ID**: E2E-043

**Objective**: Verify handling of wrong data types.

**Steps**:
1. Return JSON with wrong types (string instead of number)
2. Verify error handling

**Expected Result**:
- Type error is caught
- Runtime validation shows warning
- Page continues to function

---

## 7. Performance Tests

### 7.1 Large Dataset Rendering

**Test ID**: E2E-050

**Objective**: Verify performance with large datasets (1000+ items).

**Steps**:
1. Create JSON with 1000 items
2. Compile and render
3. Measure render time

**Expected Result**:
- Page renders within 2 seconds
- No browser freezing
- Scroll performance is smooth

**Metrics**:
- Time to first render
- Total render time
- Memory usage
- FPS during scroll

---

### 7.2 Multiple Data Sources

**Test ID**: E2E-051

**Objective**: Verify performance with multiple concurrent data fetches.

**Steps**:
1. Define 5+ data sources
2. Load page
3. Monitor network waterfall

**Expected Result**:
- Requests are parallelized where possible
- Page doesn't block during loading
- All data loads successfully

---

### 7.3 Memory Leaks

**Test ID**: E2E-052

**Objective**: Verify no memory leaks on repeated data updates.

**Steps**:
1. Configure reactive data source
2. Update data 100 times
3. Monitor memory usage

**Expected Result**:
- Memory usage stays stable
- Old DOM nodes are garbage collected
- No detached DOM tree warnings

---

## 8. Browser Compatibility Tests

### 8.1 Chrome/Chromium

**Test ID**: E2E-060

**Objective**: Verify all functionality works in Chrome.

**Steps**: Run all core tests in Chrome

**Expected Result**: All tests pass

---

### 8.2 Firefox

**Test ID**: E2E-061

**Objective**: Verify all functionality works in Firefox.

**Steps**: Run all core tests in Firefox

**Expected Result**: All tests pass

---

### 8.3 Safari

**Test ID**: E2E-062

**Objective**: Verify all functionality works in Safari.

**Steps**: Run all core tests in Safari

**Expected Result**: All tests pass

---

## Test Automation

### Playwright Example

```javascript
// tests/e2e/basic-binding.spec.js
import { test, expect } from '@playwright/test';

test('loads data and renders products', async ({ page }) => {
    await page.goto('http://localhost:3000/test-pages/basic-binding.html');

    // Wait for data to load
    await page.waitForSelector('product-card');

    // Verify count
    const cards = await page.locator('product-card').all();
    expect(cards.length).toBe(5);

    // Verify first card content
    const firstName = await cards[0].locator('[slot="name"]').textContent();
    expect(firstName).toBe('Product 1');

    // Verify no console errors
    const errors = [];
    page.on('console', msg => {
        if (msg.type() === 'error') errors.push(msg.text());
    });
    expect(errors.length).toBe(0);
});

test('transitions from loading to ready state', async ({ page }) => {
    await page.goto('http://localhost:3000/test-pages/state-machine.html');

    // Check loading state
    const gallery = page.locator('.gallery');
    await expect(gallery).toHaveAttribute('data-state', 'loading');

    // Wait for ready state
    await expect(gallery).toHaveAttribute('data-state', 'ready', { timeout: 5000 });

    // Verify data is rendered
    const items = await page.locator('item-card').all();
    expect(items.length).toBeGreaterThan(0);
});
```

---

## Manual Test Checklist

For manual testing, follow this checklist:

- [ ] Page loads without errors
- [ ] All data sources fetch successfully
- [ ] Correct number of items are rendered
- [ ] Text content matches JSON data
- [ ] Image src attributes are correct
- [ ] Data attributes are set on elements
- [ ] Loading state appears first
- [ ] Ready state appears after load
- [ ] Empty state works with empty data
- [ ] Error state works on network failure
- [ ] Scroll animations trigger
- [ ] Hover animations work
- [ ] Stagger delays are visible
- [ ] Performance is acceptable with large datasets
- [ ] localStorage loading works
- [ ] Computed data filters correctly
- [ ] Nested properties work
- [ ] Special variables ($_index, $_first) work
- [ ] Filters (currency, uppercase, etc.) apply correctly

---

## Continuous Integration

### CI Pipeline Configuration

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3

      - name: Install Playwright
        run: npm install -D @playwright/test

      - name: Install Playwright browsers
        run: npx playwright install

      - name: Compile test pages
        run: cargo run -- compile tests/e2e/fixtures/*.st

      - name: Start test server
        run: |
          npm install -g http-server
          http-server tests/e2e/pages -p 3000 &

      - name: Run Playwright tests
        run: npx playwright test

      - name: Upload test results
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: playwright-report/
```

---

## Success Criteria

The E2E test suite is considered successful when:

1. All core functionality tests pass
2. All browser compatibility tests pass
3. No console errors during normal operation
4. Performance meets acceptable thresholds
5. Error scenarios are handled gracefully
6. Manual smoke tests pass in all major browsers

---

## Future Enhancements

- Visual regression testing with Percy or Chromatic
- Accessibility testing with axe-core
- Performance budgets and monitoring
- Real user monitoring (RUM) integration
- Cross-browser automated testing with BrowserStack
- Mobile device testing
