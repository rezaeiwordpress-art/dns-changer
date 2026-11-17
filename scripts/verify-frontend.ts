import { en } from '../src/localization/en.ts';
import { fa } from '../src/localization/fa.ts';
import { formatTemplate } from '../src/localization/index.ts';
import { normalizeCommandError } from '../src/services/tauriClient.ts';
import type { ErrorCode } from '../src/types/dns.ts';
import {
  BUILT_IN_DNS_PROVIDERS_REFERENCE,
  hasIdenticalDnsPair,
  normalizeDnsDigits,
  validateIpv4InputKey,
} from '../src/utils/validation.ts';

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(`Assertion failed: ${message}`);
  }
}

function collectKeys(obj: Record<string, unknown>, prefix = ''): string[] {
  const keys: string[] = [];
  for (const [k, v] of Object.entries(obj)) {
    const full = prefix ? `${prefix}.${k}` : k;
    if (v !== null && typeof v === 'object' && !Array.isArray(v)) {
      keys.push(...collectKeys(v as Record<string, unknown>, full));
    } else {
      keys.push(full);
    }
  }
  return keys.sort();
}

// 1. IPv4 Validation & Persian/Arabic-Indic Digit Normalization Tests
for (const valid of ['1.1.1.1', '8.8.4.4', '192.168.1.1', '127.0.0.1', '۱.۱.۱.۱', '٨.٨.٤.٤']) {
  assert(validateIpv4InputKey(valid) === null, `Expected valid IPv4: ${valid}`);
}

assert(normalizeDnsDigits('۱.۰.۰.۱') === '1.0.0.1', 'Persian digits must normalize to ASCII');
assert(normalizeDnsDigits('١٩٢.١٦٨.١.١') === '192.168.1.1', 'Arabic-Indic digits must normalize to ASCII');
assert(hasIdenticalDnsPair('1.1.1.1', '1.1.1.1'), 'Identical DNS pair must be rejected');
assert(hasIdenticalDnsPair('1.1.1.1', '۱.۱.۱.۱'), 'Digit-equivalent identical DNS pair must be rejected');
assert(!hasIdenticalDnsPair('1.1.1.1', ''), 'Empty optional secondary DNS must be allowed');
assert(!hasIdenticalDnsPair('1.1.1.1', '1.0.0.1'), 'Distinct DNS pair must be allowed');

const invalidCases: Array<[string, string]> = [
  ['', 'emptyOrWhitespace'],
  [' 1.1.1.1', 'emptyOrWhitespace'],
  ['1.1.1', 'invalidFormat'],
  ['01.1.1.1', 'leadingZeros'],
  ['256.1.1.1', 'octetOutOfRange'],
  ['0.0.0.0', 'unspecifiedAddress'],
  ['۰.۰.۰.۰', 'unspecifiedAddress'],
  ['255.255.255.255', 'broadcastAddress'],
  ['224.0.0.1', 'multicastAddress'],
];

for (const [ip, expectedKey] of invalidCases) {
  const actual = validateIpv4InputKey(ip);
  assert(
    actual === expectedKey,
    `Expected '${ip}' -> '${expectedKey}', got '${String(actual)}'`,
  );
}

// 2. Built-in Provider Catalog Integrity (8 providers)
assert(
  BUILT_IN_DNS_PROVIDERS_REFERENCE.length === 8,
  'Expected exactly 8 built-in providers',
);
for (const provider of BUILT_IN_DNS_PROVIDERS_REFERENCE) {
  assert(provider.isBuiltIn, `Provider ${provider.id} must be built-in`);
  assert(
    validateIpv4InputKey(provider.primaryDns) === null,
    `Invalid primary DNS for ${provider.id}`,
  );
  assert(
    provider.secondaryDns !== null &&
      validateIpv4InputKey(provider.secondaryDns) === null,
    `Invalid secondary DNS for ${provider.id}`,
  );
  assert(
    Boolean(fa.providers.providerDescriptions[provider.id]),
    `Missing Persian description for ${provider.id}`,
  );
  assert(
    Boolean(en.providers.providerDescriptions[provider.id]),
    `Missing English description for ${provider.id}`,
  );
}

// 3. Localization Parity (fa vs en)
const faKeys = collectKeys(fa as unknown as Record<string, unknown>);
const enKeys = collectKeys(en as unknown as Record<string, unknown>);
assert(
  faKeys.length === enKeys.length &&
    faKeys.every((key, idx) => key === enKeys[idx]),
  'Persian (fa) and English (en) translation keys must match identically',
);

const allErrorCodes: ErrorCode[] = [
  'ADAPTER_NOT_FOUND',
  'ADAPTER_DISABLED',
  'ADAPTER_DISCONNECTED',
  'INVALID_DNS_ADDRESS',
  'INVALID_ADAPTER_GUID',
  'ADMIN_ACCESS_DENIED',
  'UAC_CANCELLED',
  'ELEVATION_FAILED',
  'POWERSHELL_EXECUTION_FAILED',
  'POWERSHELL_BLOCKED',
  'POWERSHELL_TIMEOUT',
  'DNS_APPLY_FAILED',
  'DNS_MODE_INDETERMINATE',
  'IPV6_MODIFICATION_DETECTED',
  'IPV6_RECOVERY_FAILED',
  'VERIFICATION_FAILED',
  'ROLLBACK_NOT_PERFORMED',
  'BACKUP_CORRUPTION',
  'BACKUP_NOT_FOUND',
  'RESTORE_FAILED',
  'NETWORK_CHANGED',
  'CONCURRENT_OPERATION',
  'DUPLICATE_PROVIDER_NAME',
  'INVALID_INPUT',
  'UNSUPPORTED_PLATFORM',
  'STORAGE_ERROR',
  'INTERNAL_ERROR',
];

for (const code of allErrorCodes) {
  assert(Boolean(fa.errors[code]?.trim()), `Missing fa error for ${code}`);
  assert(Boolean(en.errors[code]?.trim()), `Missing en error for ${code}`);
}

assert(
  Boolean(fa.common.copy?.trim()) && Boolean(en.common.copy?.trim()),
  'Missing copy translation key',
);
assert(
  Boolean(fa.common.copied?.trim()) && Boolean(en.common.copied?.trim()),
  'Missing copied translation key',
);

assert(
  formatTemplate(en.dialogs.confirmRestoreMessage, { adapter: 'Wi-Fi' }).includes(
    'Wi-Fi',
  ),
  'Template interpolation must substitute placeholders',
);

// 4. Error Normalization & Technical Details Forwarding
const tauriErr = normalizeCommandError({
  code: 'POWERSHELL_EXECUTION_FAILED',
  message: 'Generic message',
  technicalDetail: 'PowerShell execution failed: stderr preview',
});
assert(
  tauriErr.code === 'POWERSHELL_EXECUTION_FAILED' &&
    tauriErr.technicalDetail === 'PowerShell execution failed: stderr preview',
  'normalizeCommandError must forward technicalDetail from Tauri CommandError',
);

const uacErr = normalizeCommandError({
  code: 'UAC_CANCELLED',
  message: 'UAC cancelled',
});
assert(
  uacErr.code === 'UAC_CANCELLED' && uacErr.technicalDetail === undefined,
  'normalizeCommandError must omit technicalDetail when absent',
);

const strErr = normalizeCommandError('Raw invocation failure');
assert(
  strErr.code === 'INTERNAL_ERROR' &&
    strErr.technicalDetail === 'Raw invocation failure',
  'normalizeCommandError must populate technicalDetail for string errors',
);

const jsErr = normalizeCommandError(new Error('Boom'));
assert(
  jsErr.code === 'INTERNAL_ERROR' &&
    jsErr.message === 'Boom' &&
    Boolean(jsErr.technicalDetail?.includes('Boom')),
  'normalizeCommandError must populate technicalDetail for Error instances',
);

console.log(
  `Frontend verification passed: ${faKeys.length} translation keys, ${BUILT_IN_DNS_PROVIDERS_REFERENCE.length} providers, ${allErrorCodes.length} error codes.`,
);
