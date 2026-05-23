export const assetUsageOptions = [
  { label: '常用', value: 'enabled' },
  { label: '不常用', value: 'disabled' },
];

export function assetUsageText(status: string) {
  return status === 'enabled' ? '常用' : '不常用';
}

export function nextAssetUsageStatus(status: string) {
  return status === 'enabled' ? 'disabled' : 'enabled';
}

export function nextAssetUsageText(status: string) {
  return status === 'enabled' ? '设为不常用' : '设为常用';
}

export function normalizeTags(values: string[]) {
  return [...new Set(values.map((tag) => tag.trim()).filter(Boolean))];
}

export function displayTags(category: string, tags: string[]) {
  return normalizeTags([category, ...tags]);
}
