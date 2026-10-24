import { Cents, percentOf } from "./money";

export interface Coupon {
  code: string;
  percentOff: number;
  minSubtotal: Cents;
  expiresAt: Date;
}

export function couponDiscount(coupon: Coupon, subtotal: Cents, now: Date = new Date()): Cents {
  if (now >= coupon.expiresAt) {
    return 0;
  }
  if (subtotal < coupon.minSubtotal) {
    return 0;
  }
  return percentOf(subtotal, coupon.percentOff);
}
