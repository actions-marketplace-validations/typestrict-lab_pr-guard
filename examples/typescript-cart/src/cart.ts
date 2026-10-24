import { Coupon, couponDiscount } from "./discounts";
import { Cents } from "./money";

export interface LineItem {
  sku: string;
  name: string;
  unitPrice: Cents;
  quantity: number;
}

export interface Totals {
  subtotal: Cents;
  discount: Cents;
  tax: Cents;
  total: Cents;
}

export class Cart {
  private readonly items = new Map<string, LineItem>();
  private coupon: Coupon | null = null;

  addItem(item: Omit<LineItem, "quantity">, quantity = 1): void {
    if (!Number.isInteger(quantity) || quantity <= 0) {
      throw new RangeError("quantity must be a positive integer");
    }
    const existing = this.items.get(item.sku);
    if (existing) {
      existing.quantity += quantity;
    } else {
      this.items.set(item.sku, { ...item, quantity });
    }
  }

  removeItem(sku: string): void {
    this.items.delete(sku);
  }

  applyCoupon(coupon: Coupon): void {
    this.coupon = coupon;
  }

  lines(): LineItem[] {
    return [...this.items.values()];
  }

  totals(taxRate: number, now: Date = new Date()): Totals {
    const subtotal = this.lines().reduce((sum, line) => sum + line.unitPrice * line.quantity, 0);
    const discount = this.coupon ? couponDiscount(this.coupon, subtotal, now) : 0;
    const tax = Math.round((subtotal - discount) * taxRate);
    return { subtotal, discount, tax, total: subtotal - discount + tax };
  }
}
