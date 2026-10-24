# @shop/cart

Shopping cart domain logic shared by the storefront and the checkout service.
All amounts are integer cents.

```ts
const cart = new Cart();
cart.addItem({ sku: "MUG-01", name: "Coffee mug", unitPrice: 1250 }, 2);
cart.applyCoupon(coupon);
const { total } = cart.totals(0.08);
```

`npm test` runs the vitest suite.
