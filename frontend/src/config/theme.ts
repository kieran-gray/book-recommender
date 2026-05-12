import { SCHEME_NAMES, schemes, type SchemeName, type ThemeTokens } from '@/styles/schemes';

export { schemes, SCHEME_NAMES };
export type { SchemeName, ThemeTokens };

export const theme = {
	default: 'rose' satisfies SchemeName,
	list: SCHEME_NAMES
} as const;
