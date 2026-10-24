import { describe, expect, it } from "vitest";

import { Cart } from "../src/cart";
import { Coupon } from "../src/discounts";
import { formatCents, toCents } from "../src/money";

const mug = { sku: "MUG-01", name: "Coffee mug", unitPrice: 12_50 };
const beans = { sku: "BEAN-1K", name: "Beans 1kg", unitPrice: 24_99 };
const now = new Date("2024-05-01T12:00:00Z");
const tenOff: Coupon = { code: "TEN", percentOff: 10, minSubtotal: 30_00, expiresAt: new Date("2024-06-01") };

describe("money", () => {
  it("parses amounts into cents", () => {
    expect(toCents("12.5")).toBe(1250);
    expect(toCents("3")).toBe(300);
    expect(() => toCents("abc")).toThrow();
  });

  it("formats cents", () => {
    expect(formatCents(1999)).toBe("$19.99");
  });
});

describe("Cart", () => {
  it("merges quantities for the same sku", () => {
    const cart = new Cart();
    cart.addItem(mug);
    cart.addItem(mug, 2);
    expect(cart.lines()).toEqual([{ ...mug, quantity: 3 }]);
  });

  it("rejects non positive quantities", () => {
    expect(() => new Cart().addItem(mug, 0)).toThrow(RangeError);
  });

  it("computes totals with coupon and tax", () => {
    const cart = new Cart();
    cart.addItem(mug);
    cart.addItem(beans);
    cart.applyCoupon(tenOff);
    expect(cart.totals(0.08, now)).toEqual({ subtotal: 37_49, discount: 3_75, tax: 2_70, total: 36_44 });
  });

  it("ignores expired coupons and small orders", () => {
    const cart = new Cart();
    cart.addItem(mug);
    cart.applyCoupon(tenOff);
    expect(cart.totals(0, now).discount).toBe(0);
    cart.addItem(beans);
    expect(cart.totals(0, new Date("2024-07-01")).discount).toBe(0);
  });
});
