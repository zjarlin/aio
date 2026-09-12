export type CounterResponse = { value: number; tenant_id: string };

export function increment(value: number): number {
  if (!Number.isSafeInteger(value) || value < 0 || value >= Number.MAX_SAFE_INTEGER) throw new Error('Invalid counter value');
  return value + 1;
}
