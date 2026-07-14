const USCC_CHARACTERS = "0123456789ABCDEFGHJKLMNPQRTUWXY";
const USCC_WEIGHTS = [1, 3, 9, 7, 1, 3, 9, 7, 1, 3, 9, 7, 1, 3, 9, 7, 1];

export function isValidUnifiedSocialCreditCode(value: string): boolean {
  const code = value.trim().toUpperCase();
  if (code.length !== 18 || [...code].some((character) => !USCC_CHARACTERS.includes(character))) {
    return false;
  }
  const checksum = USCC_WEIGHTS.reduce(
    (sum, weight, index) => sum + USCC_CHARACTERS.indexOf(code[index]) * weight,
    0,
  );
  return USCC_CHARACTERS[(31 - (checksum % 31)) % 31] === code[17];
}

export function validateSupplierQualificationFields(input: {
  unifiedSocialCreditCode: string;
  contactName: string;
}): void {
  if (!input.unifiedSocialCreditCode.trim()) throw new Error("统一社会信用代码必填");
  if (!isValidUnifiedSocialCreditCode(input.unifiedSocialCreditCode)) {
    throw new Error("统一社会信用代码格式错误");
  }
  if (!input.contactName.trim()) throw new Error("联系人必填");
}
