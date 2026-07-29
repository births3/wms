/** UUID 校验唯一来源：宽松版 8-4-4-4-12 十六进制，不限制版本/变体位。 */
export function isUuid(value: string): boolean {
  return /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value);
}
