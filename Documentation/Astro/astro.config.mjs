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
			// sidebar: [
			// 	{
			// 		label: 'Guides',
			// 		items: [
			// 			// Each item here is one entry in the navigation menu.
			// 			{ label: 'Example Guide', slug: 'guides/example' },
			// 			{ label: 'Second Guide', slug: 'guides/faq' },
			// 		],
			// 	},
			// 	{
			// 		label: 'Reference',
			// 		autogenerate: { directory: 'reference' },
			// 	},
			// ],
			customCss: [
				'./src/styles/custom.css',
			],
		}),
	],
});
