import { Language } from '../../services/language.types';

export const content: Record<Language, any> = {
  EN: {
    title_01: {
      module_01: {
				section_01 : {
					h1: 'What is ANT?',
					d1: 'ANT on the system (Archive Nesting Technique on the System) is a hierarchical organizational method designed to streamline the storage, easier to manage and find files.',
				},
				section_02: {
					h1: 'How works?',
				}
      },
    },
    title_02: {
      module_01: {
				section10: {
					h1: '1. Levels and Directory Structure',
					p1: 'Levels refer to the depth within the folder tree directory.',
					p2: '<strong>Control Structure</strong>',
					d1: '<strong>Tip</strong>: The basic structure suggests 3 levels, although in some cases there may be more. However, it is recommended not to have too many levels and to use them only in certain cases.'
				},
				section11: {
					h1: '1.1 The Level 1 - Main Categories',
					p1: 'Is the highest hierarchy, inside from your Root Storage Folder, content the main categories;',
					d1: '<strong>Important</strong>: Use an incremental enumeration with three digits (100, 200, 300, ...), followed by a dot, followed by the Main Category name in Capitals.',
				},
				section12: {
					h1: '1.2 The Levels 2 - Categories (Subjects)',
					p1: 'Is the second hierarchy, inside from your Root Storage Folder, content the subjects;',
					p2: 'Is inside of the main category;',
					d1: '<strong>Important</strong>: For each directory add an Incremental enumeration with two digits, followed by a dot, followed by the Categories Name.',
				},
				section13: {
					h1: '1.3 The Levels 3 - Subcategories (Units, Theme, Topics, etc.)',
					p1: 'There are two ways to structure; <br>If the Directory stay for a long time should be use a <strong>Chill Structure</strong>, Otherwise if it\'s a Temporary Directory (it\'ll only be used for a short period), you should be use a <strong>control structure</strong>.',
				},
				section13_1: {
					h1: '1.3.1 Chill Structure',
					p1: 'Is simple, It has an incremental enumeration with two or more digits, tho it doesn\'t necesary go at the beginning, as it\'ll depend on the needs/customization of each one.',
					p2: 'You can use if:',
					ul1_li1: 'Had or Need a static sequence (eg. 01.Name 02.Name 03.Name ...)',
					ul1_li2: 'The directory will remain for a long time or permanently',
					d1: '<strong>Note</strong>: That the way you organize will determine whether your support folders come before or after. Remember, the order always be Symbols-Numbers-Letters."',
				},
				section13_2: {
					h1: '1.3.2 Control Structure',
					p1: 'It\'s ideal for projects, presentations, database, homeworks, etc. Use a naming convetion for controling and easy identification',
					d1: '<strong>Important</strong>: It has a naming convention, here is the meaning of each part;',
					table1_head: [
						{ col_1: 'Part', col_2: 'Meaning' },
					],
					table1_content: [
						{ col_1: 'Identificator', col_2: 'It\'s a code label that identifies it.' },
						{ col_1: 'Traker', col_2: 'It\'s your tracking index, if you use.' },
						{ col_1: 'Date', col_2: 'The format is \'YY-MM-DD\', within parentheses' },
						{ col_1: 'Breadcrumb', col_2: 'Use current or nearby folder category name.' },
						{ col_1: 'Holder', col_2: 'Refers to client, organization, school, or reference name, within brackets' },
						{ col_1: 'Name', col_2: 'Is the proj, pres, DB, hw name.' },
						{ col_1: 'Keywords', col_2: 'Are descriptive words or phrases. @@@@@' }
					],
					d2: '<strong>Tip</strong>: It\'s not necessary to have all the fields, tho follow in that order, (adapt it to your needs).',
					p2: 'You can use if:',
					ul1_li1: 'You Need to have control',
					ul1_li2: 'The data is temporary and will most likely be deleted later',
					ul1_li3: 'Share with more people if you',
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
					d1: "<strong>Important</strong>: Use the identifier, followed by a dot, followed by the name of the current path or similar (preferably write it in kebab-case)"
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
					d1: "<strong>Note</strong>: It's only for your Final Files. Don't create one folder for each one.",
					p3: "Usage Suggestions:",
				},
				section25: {
					h1: "Temporal Folder (~)",
					p1: "Its unique identifier is '~'. so you'll probably find it as '~.temporal', '~temp', '~agascfsa'",
					d1: "<strong>Note</strong>: It doesn't really need an identifier, but this way, you'll know to delete it when it's empty.",
					d2: "You can also call it '~temp', 'temporal', anyway, just remember delete it.",
					p2: "This is a folder that will help you organize a couple of files without adding them to 'temporal' and be easy to identify.",
					p3: "Because those files either don't belong at that level or are needed immediately.",
					d3: "The reason was the symbol '~' is for the organizacion ASCI then this appears in the begin.",
				},
				section26: {
					h1: "Workboard Folder (PJ, PR, PP, HW)",
					p1: "Its could be different identifier.",
					p2: "It helps organize your work-related files into distinct categories such as:",
					ul1_li1: 'need a static sequence',
					ul1_li2: 'the directory is a permanent category',
					ul1_li3: 'Share with more people if you',
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
					d1: '<strong>Tip</strong>: La estructura basica sugiere 3 niveles, aunque en podrías tener más. Sin embargo, es recomendable no tener demasiados niveles y usarlos solo si son necesarios.'
				},
				section11: {
					h1: '1.1 Nivel 1 - Categorías principales',
					p1: 'Es la jerarquía más alta, dentro de tu Carpeta de Almacenamiento Raíz, contiene las categorías principales;',
					d1: "<strong>Importante</strong>: Usa una enumeración incremental con tres dígitos (100, 200, 300, ...), seguida de un punto, seguida del nombre de la Categoría Principal en mayúsculas.",
				},
				section12: {
					h1: '1.2 Nivel 2 - Categorías (Temas)',
					p1: 'Es la segunda jerarquía, dentro de tu Carpeta de Almacenamiento Raíz, contiene los temas;',
					d1: '<strong>Importante</strong>: Para cada directorio, agrega una enumeración incremental con dos dígitos, seguida de un punto, seguida del nombre de la Categoría.',
				},
				section13: {
					h1: '1.3 Nivel 3 - Categorías (Unidades, Tema, Temas, etc.)',
					p1: 'Hay dos formas de estructurar; Si el Directorio se mantiene por mucho tiempo, se debe usar una <strong>Estructura Fria</strong>, de lo contrario, si es un Directorio Temporal (solo se usará por un período corto), se debe usar una <strong>Estructura de Control</strong>.',
				},
				section13_1: {
					h1: '1.3.1 Estructura Fria',
					p1: 'Es simple, tiene una enumeración incremental con dos o más dígitos, aunque no necesariamente va al principio, ya que dependerá de las necesidades/personalizaciones de cada quien.',
					p2: 'Puedes usarla si:',
					ul1_li1: 'Necesitas o ya tiene una secuencia estatica (ej. 01.Nombre 02.Nombre 03.Nombre ...)',
					ul1_li2: 'La carpeta permanecera para un tiempo largo o permanentemente',
					d1: '<strong>Nota</strong>: La forma en que organizas determinará si tus carpetas de soporte vienen antes o después. Consulta <a href="https://ticsw.gitbooks.io/talleres/content/BigO_Sort/tipos_de_ordenamientos.html#Ordenamiento%20Lexicográfico" target="_blank"><u><i>prioridad de orden lexicográfico</u></i></a>',
				},
				section13_2: {
					h1: '1.3.2 Estructura de Control',
					p1: 'Es ideal para proyectos, presentaciones, bases de datos, tareas, etc. Se utiliza una convención de nombres para controlar y facilitar su identificación.',
					d1: '<strong>Importante</strong>: Tiene una convención de nombres, enseguida se muestra una tabla de referencia',
					table1_head: [
						{ col_1: 'Parte', col_2: 'Significado' },
					], 
					table1_content: [
						{ col_1: 'Identificador', col_2: 'Es una etiqueta de código que lo identifica.' },
						{ col_1: 'Traker', col_2: 'Es tu índice de seguimiento (si lo usas).' },
						{ col_1: 'Fecha', col_2: "La fecha con formato 'AA-MM-DD', va entre paréntesis." },
						{ col_1: 'Breadcrumb', col_2: 'Usa el nombre de la categoría de la carpeta actual o cercana.' },
						{ col_1: 'Holder', col_2: 'Se refiere al cliente, organización, escuela o nombre de referencia, va entre corchetes.' },
						{ col_1: 'Nombre', col_2: 'Es el nombre del proyecto, presentación, base de datos o tarea.' },
						{ col_1: 'Keywords', col_2: 'Son palabras o frases descriptivas, palabras clave.' }
					],
					d2: '<strong>Tip</strong>: No es necesario rellenar todos los campos, solo continuar con el orden.',
					p2: 'Puedes usarla si:',
					ul1_li1: 'Requieres un control',
					ul1_li2: 'La información se mantendra por mucho tiempo',
					ul1_li3: 'Compartiras con mas de una persona',
				}
			},
			module_02: {
				section20: {
					h1: 'Carpetas de Soporte',
					p1: 'Estas carpetas fueron diseñadas que te sirvan de ayuda en el nivel donde estes. En ese sentido Ayuda al usuario. Cada una cuenta con un idenficador unico, que lo hace diferenciarse y de rapida identificación.'
				},
				section21: {
					h1: 'Carpeta de Configuracion (99)',
					p1: "Su identificador unico es '99', probablemente lo encontras de un '99.Configuracion'",
					p2: "Esta carpeta por lo general deberia contener configuracion de tu 'ANT', tus programas, configuraciones de Interfaces, y tu Sistema Operativo.",
				},
				section22: {
					h1: 'Carpeta de Revision (98)',
					p1: '...',
					p2: '...',
					p3: '...',
					d1: '...'
				},
				section23: {
					h1: 'Carpeta de Recursos (90)',
					p1: '...',
					p2: '...',
					p3: '...'
				},
				section24: {
					h1: 'Carpeta Archivadora (00)',
					p1: '...',
					p2: '...',
					d1: '<strong>Nota</strong>:',
					p3: '...'
				},
				section25: {
					h1: 'Carpeta Temporal (~)',
					p1: '...',
					d1: '<strong>Nota</strong>: ',
					d2: '<strong>Tip</strong>:',
					p2: '...',
					p3: '...',
					d3: '...'
				},
				section26: {
					h1: 'Espacio de Trabajo (Tarea, Proyecto, Presentación)',
					p1: '...',
					p2: '...',
					ul1_li1: '...',
					ul1_li2: '...',
					ul1_li3: '...',
				}
			}
		}
  }
};