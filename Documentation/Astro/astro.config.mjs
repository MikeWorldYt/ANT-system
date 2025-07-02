// @ts-check
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'Docs',
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
						autogenerate: { directory: 'foundationals' },
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
