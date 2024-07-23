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
					p1: 'Is the highest hierarchy, inside from your Root Storage Folder, content the main categories;',
					d1: 'Important: Use an incremental enumeration with three digits (100, 200, 300, ...), followed by a dot, followed by the Main Category name in Capitals',
				},
				section12: {
					h1: '1.2 The Levels 2 - Categories (Subjects)',
					p1: 'Is the second hierarchy, inside from your Root Storage Folder, content the subjects;',
					d1: 'For each directory add an Incremental enumeration with two digits, followed by a dot, followed by the Categories Name',
				},
				section13: {
					h1: '1.3 The Levels 3 - Subcategories (Units, Theme, Topics, etc.)',
					p1: 'There are two ways to structure; If the Directory stay for a long time should be use a <strong>Chill Structure</strong>, Otherwise if it\'s a Temporary Directory (it\'ll only be used for a short period), you should be use a <strong>control structure</strong>.',
				},
				section13_1: {
					h1: '1.3.1 Chill Structure',
					p1: 'Is simple, It has an incremental enumeration with two or more digits, tho it doesn\'t necesary go at the beginning, as it\'ll depend on the needs/customization of each one.',
					p2: 'You can use if:',
					l1: '<ul>  <li>need a static sequence</li>  <li>the directory is a permanent category</li>  </ul>',
					d1: 'Note: That the way you organize will determine whether your support folders come before or after. Remember, the order always be Symbols-Numbers-Letters."',
				},
				section13_2: {
					h1: '1.3.2 Control Structure',
					p1: 'It\'s ideal for projects, presentations, database, homeworks, etc. Use a naming convetion for controling and easy identification',
					d1: 'Important: It has a naming convention, here is the meaning of each part;',
					d2: ' It\'s not necessary to have all the fields, tho follow in that order, (adapt it to your needs).',
					p2: 'You can use if:',
					l1: '<ul>  <li>need a static sequence</li>  <li>the directory is a permanent category</li>  <li>Share with more people if you</li>  </ul>',
				}
      },
			module_02: {

				section20: {
					h1: 'Support Folders',
					p1: 'The support folders are designed to be helpful at the current level. In that sense support the user. Each has a unique identifier, making it easy to differentiate and quickly identify them.'
				},
				section21: {
					h1: "Settings Folder (99)",
					p1: "Its unique identifier is '99', so you'll probably find it as '99.Settings'",
					p2: "This folder contains general configurations for 'ANT', for your programs, interfaces, or Operating System."				
				},
				section22: {
					h1: "Review Folder (98)",
					p1: "Its unique identifier is '98', so you'll probably find it as '98.math', '98.marketing' or '98.instagram'.",
					p2: "This folder contains all your files and directories that have no classification, need renaming, or maybe are not from the current level.",
					p3: "In other words, they need to be reviewed before being into the system, and this folder helps you avoid mixing them with your other files.",
					d1: "Use the identifier, followed by a dot, followed by the name of the current path or similar (preferably write it in kebab-case)"
				},
				section23: {
					h1: "Resources Folder (90)",
					p1: "It's unique identifier is '90'. so you'll probably find it as '90.Resources'",
					p2: "This folder helps organize those files that due to their nature; are frequently needed, provide guidelines, are presets, for use.",
					p3: "Usage Suggestions:",
				},
				section24: {
					h1: "Archive Folder (00)",
					p1: "Its unique identifier is '00', so you'll can use it as '00.Final', '00.Ready', '00.Renders', '00.forProduction', etc.",
					p2: "This folder it's meant contain your deliverables or already delivered items, e.g. for your projects, homeworks, presentations, or any completed job, as well as each of its subsequent versions.",
					d1: "It's only for your Final Files. Don't create one folder for each one.",
					p3: "Usage Suggestions:",
				},
				section25: {
					h1: "Temporal Folder (~)",
					p1: "Its unique identifier is '~'. so you'll probably find it as '~.temporal', '~temp', '~agascfsa'",
					p2: "This is a folder that will help you organize a couple of files without adding them to 'temporal' and be easy to identify.",
					p3: "Because those files either don't belong at that level or are needed immediately.",
					p4: "The reason was the symbol '~' is for the organizacion ASCI then this appears in the begin.",
					d1: "It doesn't really need an identifier, but this way, you'll know to delete it when it's empty.",
					d2: "You can also call it '~temp', 'temporal', anyway, just remember delete it."
				},
				section26: {
					h1: "Workboard Folder (PJ, PR, PP, HW)",
					p1: "Its could be different identifier.",
					p2: "It helps organize your work-related files into distinct categories such as",
					l1: "<ul><li>PJ for project</li><li>PR for final project</li><li>PP for presentation</li><li>HW for homework</li></ul>"				
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