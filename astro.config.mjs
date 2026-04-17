// @ts-check
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';
import codeTheme from './src/styles/code-theme.json';
import achGrammar from './src/styles/achronyme.tmLanguage.json';

// https://astro.build/config
export default defineConfig({
	site: 'https://achrony.me',
	vite: {
		plugins: [tailwindcss()],
	},
	integrations: [
		mdx(),
		sitemap({
			// EN is the canonical variant; ES is the alternate. Keeps
			// Google from treating translated pages as duplicate content.
			i18n: {
				defaultLocale: 'en',
				locales: { en: 'en-US', es: 'es-MX' },
			},
		}),
	],
	markdown: {
		shikiConfig: {
			theme: codeTheme,
			langs: [
				{ ...achGrammar, name: 'ach', aliases: ['achronyme'] },
			],
		},
	},
});
