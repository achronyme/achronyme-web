// @ts-check
import { defineConfig } from 'astro/config';
import mdx from '@astrojs/mdx';
import tailwindcss from '@tailwindcss/vite';
import codeTheme from './src/styles/code-theme.json';
import achGrammar from './src/styles/achronyme.tmLanguage.json';

// https://astro.build/config
export default defineConfig({
	site: 'https://achrony.me',
	vite: {
		plugins: [tailwindcss()],
	},
	integrations: [mdx()],
	markdown: {
		shikiConfig: {
			theme: codeTheme,
			langs: [
				{ ...achGrammar, name: 'ach', aliases: ['achronyme'] },
			],
		},
	},
});
