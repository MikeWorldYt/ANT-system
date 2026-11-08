// @ts-check
import starlight from '@astrojs/starlight';
import { defineConfig } from 'astro/config';

export default defineConfig({
	site: 'https://ant-system.vercel.app',
	// Starlight has no root locale, so nothing is built at "/"
	redirects: {
		'/': '/en/',
	},
	integrations: [
		starlight({
			title: 'ANT System',
			logo: {
				src: './src/assets/ant-logo.svg',
				alt: 'ANT System logo',
			},
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
							{ label: 'Downloads', link: 'start-here/download', translations: { es: 'Descargas' } },
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
							{ label: 'Naming Conventions', link: 'foundationals/naming-conventions', translations: { es: 'Nomenclaturas' } },
							{ label: 'Support Nodes', link: 'foundationals/support-nodes', translations: { es: 'Nodos de Soporte' } },
							{ label: 'Workboard Nodes', link: 'foundationals/workboard-nodes', translations: { es: 'Nodos del Tablero de Trabajo' } },
						]
					},
					{
						label: 'Folder Manager',
						translations: { es: 'Folder Manager' },
						items: [
							{ label: 'Overview', slug: 'folder-manager', translations: { es: 'Resumen' } },
							{ label: 'Views and sorting', slug: 'folder-manager/views', translations: { es: 'Vistas y orden' } },
							{ label: 'Working with folders', slug: 'folder-manager/folders', translations: { es: 'Trabajar con carpetas' } },
							{ label: 'Project Management', slug: 'folder-manager/project-management', translations: { es: 'Gestión de Proyectos' } },
							{ label: 'Search', slug: 'folder-manager/search', translations: { es: 'Búsqueda' } },
							{ label: 'Keyboard shortcuts', slug: 'folder-manager/shortcuts', translations: { es: 'Atajos de teclado' } },
							{ label: 'Settings', slug: 'folder-manager/settings', translations: { es: 'Ajustes' } },
							{ label: 'Backups', slug: 'folder-manager/backups', translations: { es: 'Respaldos' } },
							{ label: 'Extensions', slug: 'folder-manager/extensions', translations: { es: 'Extensiones' } },
							{ label: 'Updates', slug: 'folder-manager/updates', translations: { es: 'Actualizaciones' } },
							{ label: 'Data and troubleshooting', slug: 'folder-manager/troubleshooting', translations: { es: 'Datos y solución de problemas' } },
						],
					},
					{
						label: 'Advanced',
						translations: { es: 'Avanzado' },
						items: [
							{ label: 'CLI', slug: 'advanced/cli', translations: { es: 'CLI' } },
							{ label: 'AI integrations', slug: 'advanced/ai-integrations', translations: { es: 'Integraciones de IA' } },
						],
					},
					{
						label: 'Reference',
						translations: { es: 'Referencias' },
						items: [{ autogenerate: { directory: 'reference' } }],
					},
			],
			customCss: [
				'./src/styles/custom.css',
			],
		}),
	],
});
