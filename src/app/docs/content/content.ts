import { Language } from '../../services/language.types';

export const content: Record<Language, any> = {
  EN: {
    title_01: {
      module_01: {
				section_01 : {
					h1: 'What is ANT?',
				}
      },
    },
    title_02: {
      module_01: {
				section10: {
					h1: '1. Levels and Directory Structure',
					p1: 'Levels refer to the depth within the folder tree directory.',
					p2: '<strong>Control Structure</strong>',
					d1: 'Tip: The basic structure suggests 3 levels, although in some cases there may be more. However, it is recommended not to have too many levels and to use them only in certain cases.'
				},
				section11: {
					h1: '1.1 The Level 1 - Main Categories',
				},
				section12: {
					h1: '1.2 The Levels 2 - Categories (Subjects)',
				},
				section13: {
					h1: '1.3 The Levels 3 - Subcategories (Units, Theme, Topics, etc.)',
				},
				section13_1: {
					h1: '1.3.1 Chill Structure',
				},
				section13_2: {
					h1: '1.3.2 Control Structure',
				}
      }
    }
  },
  ES: {
    title_01: {
			module_01: {
				section_01 : {
					h1: '¿Que es ANT?',
				}
			},
		},
		title_02: {
			module_01: {
				section10: {
					h1: '1. Estructura de Niveles y Carpetas',
					p1: 'Por <strong>niveles</strong> significa la profundidad dentro del arbol de carpetas.',
					d1: 'Tip: La estructura basica sugiere 3 niveles, aunque en podrías tener más. Sin embargo, es recomendable no tener demasiados niveles y usarlos solo si son necesarios.'
				},
				section11: {
					h1: '1.1 Nivel 1 - Categorías principales',
				},
				section12: {
					h1: '1.2 Nivel 2 - Categorías (Temas)',
				},
				section13: {
					h1: '1.3 Nivel 3 - Categorías (Unidades, Tema, Temas, etc.)',
				},
				section13_1: {
					h1: '1.3.1 Estructura Sencilla',
				},
				section13_2: {
					h1: '1.3.2 Estructura de Control',
				}
			}
		}
  }
};