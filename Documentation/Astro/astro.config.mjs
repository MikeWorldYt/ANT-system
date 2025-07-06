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
					{
						label: 'Start Here',
						items: [
							{ label: 'Getting Started', link: 'start-here/getting-started' },
							{ label: 'Migrating Plan', link: 'start-here/migrating-plan' },
						],
					},
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
							{ label: 'Workboard Nodes', link: 'foundationals/workboard-nodes' },
							{ label: 'Support Nodes', link: 'foundationals/support-nodes' },
							{ label: 'Nomenclature', link: 'foundationals/nomenclature' },
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
