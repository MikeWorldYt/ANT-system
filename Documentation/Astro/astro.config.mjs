// @ts-check
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

export default defineConfig({
	integrations: [
		starlight({
			title: 'Docs',
			defaultLocale: 'en',
			locales: {
				en: { label: 'English', },
				es: { label: 'Español', },
			},
			favicon: '/favicon.ico',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/MikeWorldYt/ANT-system/' },
				{ icon: 'linkedin', label: 'Linkedin', href: 'https://www.linkedin.com/in/mikeworldyt/' },
			],
			sidebar: [
				
					{
						label: 'Start Here',
						translations: { es: 'Comienza Aquí' },
						items: [
							{ label: 'Getting Started', link: 'start-here/getting-started', translations: { es: 'Primeros Pasos' } },
							{ label: 'Migrating Plan', link: 'start-here/migrating-plan', translations: { es: 'Plan de Migración' } },
						],
					},
					{
						label: 'Introduction',
						translations: { es: 'Introducción' },
						items: [
							// Each item here is one entry in the navigation menu.
							{ label: 'What is ANT?', slug: 'introduction/what-is-ant', translations: { es: '¿Qué es ANT?' } },
							// { label: 'Second Guide', slug: 'guides/faq' },
						],
					},
					{
						label: 'Foundationals',
						translations: { es: 'Fundamentos' },
						//autogenerate: { directory: 'foundationals' },
						items: [
							{ label: 'Core Concepts', link: 'foundationals/core-concepts', translations: { es: 'Conceptos Básicos' } },
							{ label: 'Folder Structure', link: 'foundationals/folder-structure', translations: { es: 'Estructura de Carpetas' } },
							{ label: 'Workboard Nodes', link: 'foundationals/workboard-nodes', translations: { es: 'Nodos del Tablero de Trabajo' } },
							{ label: 'Support Nodes', link: 'foundationals/support-nodes', translations: { es: 'Nodos de Soporte' } },
							{ label: 'Nomenclature', link: 'foundationals/nomenclature', translations: { es: 'Nomenclatura' } },
						]
					},
					{
						label: 'Reference',
						translations: { es: 'Referencias' },
						autogenerate: { directory: 'reference' },
					},
			],
			customCss: [
				'./src/styles/custom.css',
			],
		}),
	],
});
