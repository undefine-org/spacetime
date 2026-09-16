@data fetch $users : "/api/users";
body {
    $cartOpen bool: false;
}
.cart-toggle {
    @on &.click { $cartOpen <- true; }
}
.card {
    &b .some-class;
    $x: &$b;
}
<div class="cart-toggle">open</div>
<div class="card">card</div>
<div class="user-list">users</div>
