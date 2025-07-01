// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwind from '@astrojs/tailwind';

// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			title: 'My Docs',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/withastro/starlight' }],
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
