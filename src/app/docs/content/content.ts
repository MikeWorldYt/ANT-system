import { Language } from '../../services/language.types';

export const content: Record<Language, any> = {
  EN: {
    title_01: {
      module_01: {
				header : {
					h3: 'Introduction',
					h1: 'What is ANT?',
					d1: 'ANT on the system (Archive Nesting Technique on the System) is a hierarchical organizational method designed to streamline the storage, easier to manage and find files.',
				},
				article_01: {
					d1: '<strong>ANT system</strong> (Archive Nesting Technique System) <br>      is a <i>hierarchical organizational method</i> designed to streamline  <br>      the storage, easier to manage and find files.',
					p1: 'Emerges in a Workplace <strong>to Manage a Large Volume of files</strong><br> and is adapted for Home use.'
				},
				article_02: {
					h1: 'How works?',
					p1: 'Her rules organize files in a way that mimics an ant colony, <br> with a categories pathways in levels and some nested paths. <br> Also, it employs a naming convention that allows the user to find a file anywhere. <br> <br>Following this method will ensure that your Digital Storage is well-organized, <br>Easily accessible, and Efficiently managed'
				},
				article_03: {
					h1: 'Key Benefits of ANT System:',
					ul1_li1: '<strong>1. Structured Organization:</strong>  Files are organized in a clear hierarchical structure, making navigation easy.',
					ul1_li2: '<strong>2. Convenient Nomenclature:</strong>  Folders and subfolders are named with an incremental and intuitive patern, improving the organization, creation, and search of content.',
					ul1_li3: "<strong>3.Clean Desktop:</strong>  A special 'Projects' folder serves as a Temporary Repository for current and future projects, eliminating the need to use the desktop for these purposes and keeping clutter-free.",
					ul1_li4: '<strong>4.Master Tracking Work/School Projects:</strong>   It allows you track ongoing projects and store ideas, When a project is done you\'ll know it, If you need files from another project you\'ll know where they are.',
					p1: ''
				}
      },
    },
    title_02: {
			header: {
				h3: '',
				h1: ''
			},
      module_01: {
				article_10: {
					id: 'Levels and Structure',
					h1: '1. Levels and Directory Structure',
					p1: " The data structure used by <strong>ANT</strong> is hierarchical, tho it's designed to be decentralized.<br> Each folder acts as a node with paths, that start at the 'root' and continue into deeper <strong><i>levels</i></strong>. <br> The term <strong><i>Levels </i></strong> refers to the depth of the folder within a tree directory.",
					d1: '<strong>Tip</strong>: The basic structure suggests 3 levels, although in some cases there may be more. However, it is recommended not to have too many levels and to use them only in certain cases.',
					d2: "<strong>Note</strong>: Directories are not exclusive or static; they can be moved to another path, or even shared with others simultaneously. In other words, they are flexible and can create associated connections that, in most cases, do not need to be renamed or copied. ",
					a1: "Read more"
				},
				article_11: {
					id: 'Level 1',
					h1: '1.1 The Level 1 - Main Categories (Contexts)',
					p1: 'Is the highest hierarchy, inside from your Root Storage Folder, content the main categories. <br> Defines a portion of space, and the usage it will have.',
					p2: "This use an incremental enumeration with three digits (100, 200, 300, ...), followed by a dot, followed by the Main Category name in Capitals. ",
					d1: '<strong>Important</strong>: This is defined the first time and should remain that way. <br> Renaming it may cause conflicts with programs that use absolute paths, requiring EVERYTHING to be relink again.',
					p3: "We recommend using the following categories, in the following order;",
					ul1_li1: '<strong>100.PERSONAL</strong>: For your personal files and documents.',
					ul1_li2: '<strong>200.WORK</strong>: For work-related files.',
					ul1_li3: '<strong>300.PROJECTS</strong>: ',
					ul1_li3_a: 'Support Directory (optional order)',
					ul1_li4: '<strong>400.ACADEMIC</strong>: For school or related files.',
					ul1_li5: '<strong>500.HOBBIES</strong>: For files related to your leisure activities.',
					a1: "Read more about <i>Support Folder \"PROJETCS\"</i>",
					d2: "Separating by contexts allows for maintaining and managing files, creating a barder that <i>if respected</i> reduces the likelihood of mixing files and helps to have a clear focus on their use and what is found there."
				},
				article_12: {
					id: 'Level 2',
					h1: '1.2 The Levels 2 - Categories (Subjects)',
					p1: 'They are found within <strong><i>Level 1 directories</i></strong>, being the second in the hierarchy.<br> They contain more specific categories, such as Area, Topic, Subject, Organization, Type of work or relationed.',
					p2: 'For each directory add an Incremental enumeration with two digits, followed by a dot, followed by the Categories Name in Capitals..',
					d1: "The order in the sequence is not essential and only facilitates a visual organization. <br> Although it may not represent a logical sequence, numbering makes it <i>more predictable</i> for the user to find what they need. However, it can be useful for prioritizing or arranging it in a custom sequence rather than alphabetically. <br> Also when a folder is delted or added, It does'nt affect his order, <br> Numering creates a control to maintain the user's customized sequence.",
				},
				article_13: {
					id: 'Level 3',
					h1: '1.3 The Levels 3 - Subcategories (Units, Theme, Topics, etc.)',
					p1: 'There are two ways to structure; <br>If the Directory stay for a long time should be use a <strong>Chill Structure</strong>, Otherwise if it\'s a Temporary Directory (it\'ll only be used for a short period), you should be use a <strong>control structure</strong>.',
				},
				article_13_1: {
					h1: '1.3.1 Chill Structure',
					p1: 'Is simple, It has an incremental enumeration with two or more digits, tho it doesn\'t necesary go at the beginning, as it\'ll depend on the needs/customization of each one.',
					p2: 'You can use if:',
					ul1_li1: 'Had or Need a static sequence (eg. 01.Name 02.Name 03.Name ...)',
					ul1_li2: 'The directory will remain for a long time or permanently',
					d1: '<strong>Note</strong>: That the way you organize will determine whether your support folders come before or after. Remember, the order always be Symbols-Numbers-Letters."',
				},
				article_13_2: {
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
						{ col_1: 'Keywords', col_2: 'Are descriptive words or phrases.' }
					],
					d2: '<strong>Tip</strong>: It\'s not necessary to have all the fields, tho follow in that order, (adapt it to your needs).',
					p2: 'You can use if:',
					ul1_li1: 'You Need to have control',
					ul1_li2: 'The data is temporary and will most likely be deleted later',
					ul1_li3: 'Share with more people if you',
				}
      },
			module_02: {

				article_20: {
					h1: '2. Support Folders',
					p1: 'The support folders are designed to be helpful at the current level. In that sense support the user. Each has a unique identifier, making it easy to differentiate and quickly identify them.'
				},
				article_21: {
					h1: "Settings Folder (99)",
					p1: "Its unique identifier should be '99', so you'll probably find it as '99.Settings'",
					p2: "This folder contains general configurations for 'ANT', for your programs, interfaces, or Operating System."				
				},
				article_22: {
					h1: "Review Folder (98)",
					p1: "Its unique identifier should be '98', so you'll probably find it as '98.math', '98.marketing' or '98.instagram'.",
					p2: "This folder contains all your files and directories that have no classification, need renaming, or maybe are not from the current level.",
					p3: "In other words, they need to be reviewed before being into the system, and this folder helps you avoid mixing them with your other files.",
					d1: "<strong>Important</strong>: Use the identifier, followed by a dot, followed by the name of the current path or similar (preferably write it in kebab-case)"
				},
				article_23: {
					h1: "Resources Folder (90)",
					p1: "It's unique identifier should be '90'. so it should be named '90.Resources'.",
					p2: "This folder helps organize those files that due to their nature; are frequently used, as brand guidelines, presets, multimedia, libraries, logos, or any other for use.",
					p3: "Usage Suggestions:",
				},
				article_24: {
					h1: "Archive Folder (00)",
					p1: "Its unique identifier should be '00', so you'll can use it as '00.Final', '00.Ready', '00.Renders', '00.forProduction', etc.",
					p2: "This folder it's meant contain your deliverables or already delivered items, e.g. for your projects, homeworks, presentations, or any completed job, as well as each of its subsequent versions.",
					d1: "<strong>Note</strong>: It's only for your Final Files. Don't create one folder for each one.",
					p3: "Usage Suggestions:",
				},
				article_25: {
					h1: "Temporal Folder (~)",
					p1: "Its unique identifier should be '~'. so you'll probably find it as '~.temporal', '~temp', '~agascfsa'",
					d1: "<strong>Note</strong>: It doesn't really need an identifier, but this way, you'll know to delete it when it's empty.",
					d2: "You can also call it '~temp', 'temporal', anyway, just remember delete it.",
					p2: "This is a folder that will help you organize a couple of files without adding them to 'temporal' and be easy to identify.",
					p3: "Because those files either don't belong at that level or are needed immediately.",
					d3: "The reason was the symbol '~' is for the organizacion ASCI then this appears in the begin.",
				},
				article_26: {
					h1: "Workboard Folder (PJ, PR, PP, HW)",
					p1: "Its could be different identifier.",
					p2: "It helps organize your work-related files into distinct categories such as:",
					ul1_li1: 'need a static sequence',
					ul1_li2: 'the directory is a permanent category',
					ul1_li3: 'Share with more people if you',
				}
			},
			module_03: {
				article_30: {
					h1: "3. Files Nomenclature",
					p1: "test lorem ipsum"
				}
			}
    }
  },
  ES: {
    title_01: {
			module_01: {
				header : {
					h3: 'Introducción',
					h1: '¿Que es ANT?',
				},
				article_01: {
					d1: '<strong>El sistema ANT</strong> (Archive Nesting Technique System) <br>      en un <i>metodo de organizacion jerarquico</i> diseñado para optimizar     <br>      el almacenamiento, administrar y encontrar archivos de forma facil y rápida.',
					p1: 'Surge en un ambiente de trabajo para <strong>gestionar un gran volumen de archivos</strong> y es adaptado para su uso domestico.',
				},
				article_02: {
					h1: '¿Como funciona?',
					p1: 'Sus reglas organizan archivos de una manera que imita una colonia de hormigas, <br> teniendo los archivos en rutas por categorías en niveles y rutas convenientemente anidadas. <br> Además, emplea una convención de nombres que permite al usuario encontrar un archivo en cualquier lugar. <br> <br>Seguir este método garantizará que tu Almacenamiento Digital esté bien organizado, <br>fácilmente accesible y eficientemente gestionado.'
				},
				article_03: {
					h1: 'Beneficios al usar ANT',
					ul1_li1: '<strong>1. Organización Estructurada:</strong> Los archivos están organizados en una estructura jerárquica clara, lo que facilita su navegación.',
					ul1_li2: '<strong>2. Nomenclatura Conveniente:</strong> Las carpetas y subcarpetas están nombradas con un patrón incremental e intuitivo, mejorando la organización, creación y búsqueda de contenido.',
					ul1_li3: "<strong>3. Escritorio Limpio:</strong> Una carpeta especial de 'Proyectos' sirve como un Repositorio Temporal para proyectos actuales y futuros, eliminando la necesidad de usar el escritorio para estos fines y manteniéndolo libre de desorden.",
					ul1_li4: "<strong>4. Seguimiento Maestro de Proyectos de Trabajo/Escuela:</strong> Permite rastrear proyectos en curso y almacenar ideas. Cuando un proyecto esté terminado, lo sabrás. Si necesitas archivos de otro proyecto, sabrás dónde están.",
				}
			},
		},
		title_02: {
			module_01: {
				article_10: {
					id: 'Estructura y Niveles',
					h1: '1. Estructura y Niveles',
					p1: "La estructura de datos que usa <strong>ANT</strong> aunque es jerarquica, esta pensada para ser decentralizada. <br> Cada carpeta son nodos con rutas, que comienzan donde la 'raiz' y continuan en <strong><i> niveles </i></strong>mas adentro. <br> Por <strong>niveles</strong> se refiere a la profundidad dentro del arbol de carpetas.",
					d1: '<strong>Tip</strong>: La estructura basica sugiere 3 niveles. Si bien podrías tener más, es recomendable no tener demasiados niveles y usarlos solo en ciertos casos.',
					d2: '<strong>Nota</strong>:Los directorios no son exclusivos o estaticos, se pueden mover a otra ruta, o incluso compartirse con otras al mismo tiempo, en otras palabras son flexibles y crear conexiones asociadas que en su mayoria de casos no necesitaran renombrase o copiarse. ',
					a1: "Leer más..."
				},
				article_11: {
					id: 'Nivel 1',
					h1: '1.1 Nivel 1 - Categorías principales (Contextos)',
					p1: 'Es la jerarquía más alta, dentro de tu Carpeta de Almacenamiento Raíz, contiene las categorías principales. <br> Define una porcion de tu espacio y el uso que tendra.',
					p2: "Usa una enumeración incremental con tres dígitos (100, 200, 300, ...), seguida de un punto, seguida del nombre de la Categoría Principal en mayúsculas. ",
					d1: "<strong>Importante</strong>: Se define la primera vez y debera permanecer asi, el renombrarlo puede causar conflictos con programas que usan rutas absolutas, teniendo que revincular TODO nuevamente.",
					p3: "Recomendamos usar las siguientes categorias, en el siguiente orden;",
					ul1_li1: '<strong>100.PERSONAL</strong>: Para tus archivos y documentos personales.',
					ul1_li2: '<strong>200.TRABAJO</strong>: Para archivos relacionados de trabajo.',
					ul1_li3: '<strong>300.PROJECTOS</strong>: Carpeta de soporte, (orden opcional).',
					ul1_li4: '<strong>400.ACADEMICO</strong>: Para archivos escolares o relacionados.',
					ul1_li5: '<strong>500.HOBBIES</strong>: Para archivos relacionados con actividades de tu tiempo libre.',
					a1: "Leer más acerca de la carpeta de soporte <i>\"PROYECTOS\"</i>",
					d2: "Separar en por contextos permite mantener y manejar archivos, crea una barrera <i>que si se respeta</i> reduce la probabilidad de mezclar archivos y ayuda a tener un enfoque claro de su uso y lo que se encuentra alli."
				},
				article_12: {
					id: 'Nivel 2',
					h1: '1.2 Nivel 2 - Categorías (Temas)',
					p1: 'Se encuentran dentro de los <strong><i>Directorios de Nivel 1</i></strong>, siendo 2da en la jerarquia,<br> contiene categorias mas especificas, por ejemplo el Area, Tema o Materia.',
					p2: "Para cada directorio, agrega una enumeración incremental con dos dígitos, seguida de un punto, seguida del nombre de la Categoría en mayúsculas.",
					d1: "El orden en la secuencia no es escencial y solo facilita su organizacion visual. <br> Si bien podria no represantar una secuencia logica el hecho de numerar hace que sea mas predecible para el usuario encontrar la que necesita, <br> Sin embargo, puede ser util para priorizar u ordenarla en una secuencia personalizada y no alfabeticamente. <br> Ademas numerarlo crea un control para mantener la secuencia personlizada del usuario.",
				},
				article_13: {
					id: 'Nivel 3',
					h1: '1.3 Nivel 3 - Categorías (Unidades, Tema, Temas, etc.)',
					p1: 'Hay dos formas de estructurar; Si el Directorio se mantiene por mucho tiempo, se debe usar una <strong>Estructura Fria</strong>, de lo contrario, si es un Directorio Temporal (solo se usará por un período corto), se debe usar una <strong>Estructura de Control</strong>.',
				},
				article_13_1: {
					h1: '1.3.1 Estructura Fria',
					p1: 'Es simple, tiene una enumeración incremental con dos o más dígitos, aunque no necesariamente va al principio, ya que dependerá de las necesidades/personalizaciones de cada quien.',
					p2: 'Puedes usarla si:',
					ul1_li1: 'Necesitas o ya tiene una secuencia estatica (ej. 01.Nombre 02.Nombre 03.Nombre ...)',
					ul1_li2: 'La carpeta permanecera para un tiempo largo o permanentemente',
					d1: '<strong>Nota</strong>: La forma en que organizas determinará si tus carpetas de soporte vienen antes o después. Consulta <a href="https://ticsw.gitbooks.io/talleres/content/BigO_Sort/tipos_de_ordenamientos.html#Ordenamiento%20Lexicográfico" target="_blank"><u><i>prioridad de orden lexicográfico</u></i></a>',
				},
				article_13_2: {
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
				article_20: {
					h1: '2. Carpetas de Soporte',
					p1: 'Estas carpetas fueron diseñadas para que te sirvan de ayuda en el nivel donde estes. En ese sentido Ayuda al usuario. Cada una cuenta con un idenficador unico, lo que la hace diferenciarse e identificarse rapidamente del resto.'
				},
				article_21: {
					h1: 'Carpeta de Configuracion (99)',
					p1: "Su identificador deberia ser '99', probablemente ya la tienes como '99.Configuracion'",
					p2: "Esta carpeta por lo general deberia contener configuracion de tu 'ANT', tus programas, configuraciones de Interfaces, y tu Sistema Operativo.",
				},
				article_22: {
					h1: 'Carpeta de Revision (98)',
					p1: "Su Identificadro deberia ser '98', probablemente lo encontraras como '98.Matematicas', '98.Marketing' o '98.Instragram'. ",
					p2: 'Este folder contiene todos tus archivos y carpetas que no tienes clasificados, necesitan renombrarse o probablemente ni pertenecen al actual nivel.',
					p3: 'En otras palabras, necesitan revision antes de ingresar al sistema, y este folder te ayuda a esto y para no mezclarlo con otros archivos.',
					d1: '<strong>Importante</strong>: Usa el identificador, seguido de un punto, seguido del nombre de la actual ruta o similar (preferentemente usa kebab-case)'
				},
				article_23: {
					h1: 'Carpeta de Recursos (90)',
					p1: "Su Identificadro deberia ser '90', asi que deberia estar nombrada como '90.Recursos'.",
					p2: 'Esa carpeta ayuda a organizar aquellos archivos que, debido a su naturaleza, son frecuentemente usados, como un manual de identidad, ajustes preestablecidos, multimedia, librerias, logotipos, o cualquier archivo que sirva como un recurso. ',
					p3: ''
				},
				article_24: {
					h1: 'Carpeta Archivadora (00)',
					p1: "...",
					p2: '...',
					d1: '<strong>Nota</strong>:',
					p3: '...'
				},
				article_25: {
					h1: 'Carpeta Temporal (~)',
					p1: '...',
					d1: '<strong>Nota</strong>: ',
					d2: '<strong>Tip</strong>:',
					p2: '...',
					p3: '...',
					d3: '...'
				},
				article_26: {
					h1: 'Espacio de Trabajo (Tarea, Proyecto, Presentación)',
					p1: '...',
					p2: '...',
					ul1_li1: '...',
					ul1_li2: '...',
					ul1_li3: '...',
				}
			},
			module_03: {
				article_30: {
					h1: "3. Nomenclatura de Archivos",
					p1: "test lorem ipsum"
				}
			}
		}
  }
};