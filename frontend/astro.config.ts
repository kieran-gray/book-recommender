import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';
import cloudflare from '@astrojs/cloudflare';
import { siteConfig } from './src/config/site';

export default defineConfig({
	site: siteConfig.url,
	output: 'server',
	vite: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		plugins: [tailwindcss() as any]
	},
	adapter: cloudflare()
});
