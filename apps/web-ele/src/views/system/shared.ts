export const statusOptions = [
  { label: '启用', value: 'enabled' },
  { label: '禁用', value: 'disabled' },
];

export function formatTime(value: number) {
  if (!value) {
    return '-';
  }
  return new Date(value).toLocaleString();
}

export function parseRoleCodes(value: string) {
  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}
