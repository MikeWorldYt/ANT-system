// @ts-check
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'Docs',
			favicon: '/favicon.ico',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/MikeWorldYt/ANT-system/' },
				{ icon: 'linkedin', label: 'Linkedin', href: 'https://www.linkedin.com/in/mikeworldyt/' },
			],
				sidebar: [
					//{ label: 'Home', link: '/' },
					{ label: 'Getting Started', link: 'getting-started' },
					{
						label: 'Introduction',
						items: [
							// Each item here is one entry in the navigation menu.
							{ label: 'What is ANT?', slug: 'introduction/what-is-ant' },
							// { label: 'Second Guide', slug: 'guides/faq' },
						],
					},
					{
						label: 'Foundationals',
						//autogenerate: { directory: 'foundationals' },
						items: [
							{ label: 'Core Concepts', link: 'foundationals/core-concepts' },
							{ label: 'Folder Structure', link: 'foundationals/folder-structure' },
						]
					},
					{
						label: 'Reference',
						autogenerate: { directory: 'reference' },
					},
				],
			customCss: [
				'./src/styles/custom.css',
			],
		}),
	],
});
