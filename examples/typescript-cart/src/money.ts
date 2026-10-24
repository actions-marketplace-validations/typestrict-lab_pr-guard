/** Amounts are integer cents so totals never drift with floating point. */
export type Cents = number;

export function toCents(amount: string): Cents {
  const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(amount.trim());
  if (!match) {
    throw new Error(`invalid amount: ${amount}`);
  }
  const fraction = (match[2] ?? "").padEnd(2, "0");
  return Number(match[1]) * 100 + Number(fraction);
}

export function formatCents(cents: Cents, currency = "USD"): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency }).format(cents / 100);
}

export function percentOf(cents: Cents, percent: number): Cents {
  return Math.round((cents * percent) / 100);
}
