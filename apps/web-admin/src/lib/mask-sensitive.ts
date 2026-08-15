/** 敏感展示：空值不占位；短值全遮；长值保留前 3 后 4。已按该规则脱敏的值再跑一遍保持不变。 */
export function maskSensitiveDisplayValue(value: string | null | undefined): string | undefined {
  const characters = [...(value?.trim() ?? "")];
  if (characters.length === 0) return undefined;
  if (characters.length <= 7) return "*".repeat(characters.length);
  return `${characters.slice(0, 3).join("")}${"*".repeat(characters.length - 7)}${characters.slice(-4).join("")}`;
}
