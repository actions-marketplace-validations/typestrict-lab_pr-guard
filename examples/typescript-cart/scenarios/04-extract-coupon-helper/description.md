Extract isCouponActive helper

Pure refactor, no behavior change. The checkout banner needs to know whether a coupon is still
valid without computing a discount, so the validity check moves into an exported
`isCouponActive` helper that `couponDiscount` reuses.
