export interface TaxQuote {
  postalCode: string;
  rate: number;
}

const DEFAULT_BASE_URL = "https://api.taxrates.example.com/v2";

export class TaxClient {
  constructor(
    private readonly apiKey: string = process.env.TAX_API_KEY ?? "",
    private readonly baseUrl: string = DEFAULT_BASE_URL,
  ) {
    if (!this.apiKey) {
      throw new Error("TAX_API_KEY is not set");
    }
  }

  async rateFor(postalCode: string): Promise<TaxQuote> {
    const response = await fetch(`${this.baseUrl}/rates/${encodeURIComponent(postalCode)}`, {
      headers: { Authorization: `Bearer ${this.apiKey}` },
    });
    if (!response.ok) {
      throw new Error(`tax service returned ${response.status}`);
    }
    const body = (await response.json()) as { rate: number };
    return { postalCode, rate: body.rate };
  }
}
