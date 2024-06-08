import os

# Make Dir of Level 1
root = 'Root_Storage'
a1 = '100.PERSONAL'
a2 = '200.WORK'
a3 = '300.PROJECTS'
a4 = '400.ACADEMIC'
a5 = '500.HOBBIES'
res = '90.Resources'
rev = '98.Review'

os.makedirs(root, exist_ok=True)
os.makedirs(f'{root}/{a1}', exist_ok=True)
os.makedirs(f'{root}/{a2}', exist_ok=True)
os.makedirs(f'{root}/{a3}', exist_ok=True)
os.makedirs(f'{root}/{a4}', exist_ok=True)
os.makedirs(f'{root}/{a5}', exist_ok=True)
os.makedirs(f'{root}/900.Resources', exist_ok=True)
os.makedirs(f'{root}/9{rev}', exist_ok=True)
os.makedirs(f'{root}/999.Setting', exist_ok=True)

# Make Dir of Level 2 and 3
		## --- 100 PERSONAL
os.makedirs(f'{root}/{a1}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/{res}', exist_ok=True)

      ### 101 MyDocuments
os.makedirs(f'{root}/{a1}/01.MyDocuments (Suggested)/{rev}', exist_ok=True)

paths_list = ["01.IdentityDocs", "02.FinancialDocs", "03.HealthDocs", ]
for path in paths_list:
	os.makedirs(f'{root}/{a1}/01.MyDocuments (Suggested)/{path} (Suggested)/{rev}', exist_ok=True)

			### 102 FamilyDocuments
os.makedirs(f'{root}/{a1}/02.FamilyDocuments (Suggested)/{rev}', exist_ok=True)

			### 103 OtherDocuments
os.makedirs(f'{root}/{a1}/03.OtherDocuments (Suggested)/{rev}', exist_ok=True)

			### 104 MyGallery
os.makedirs(f'{root}/{a1}/04.MyGallery (Suggested)/01.Family(Suggested)', exist_ok=True)
os.makedirs(f'{root}/{a1}/04.MyGallery (Suggested)/02.Friends(Suggested)', exist_ok=True)

			### 105 Curriculum
os.makedirs(f'{root}/{a1}/05.Curriculum (Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/05.Curriculum (Suggested)/01.Networking(Suggested)', exist_ok=True)
os.makedirs(f'{root}/{a1}/05.Curriculum (Suggested)/02.Portfolio(Suggested)', exist_ok=True)
os.makedirs(f'{root}/{a1}/05.Curriculum (Suggested)/03.Certificates(Suggested)', exist_ok=True)

		## --- 200 WORK
os.makedirs(f'{root}/{a2}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a2}/{res}', exist_ok=True)

			### --- 201 <edit>
os.makedirs(f'{root}/{a2}/01.Category(Suggested)/{rev}', exist_ok=True)

			### --- 202 <edit>
os.makedirs(f'{root}/{a2}/02.Category(Suggested)/{rev}', exist_ok=True)

			### --- 203 <edit>
os.makedirs(f'{root}/{a2}/03.Category(Suggested)/{rev}', exist_ok=True)

		## --- 300 PROJECTS
os.makedirs(f'{root}/{a3}/{rev}', exist_ok=True)

		## --- 400 ACADEMIC
os.makedirs(f'{root}/{a4}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a4}/{res}', exist_ok=True)
os.makedirs(f'{root}/{a4}/{res}/01.SchoolAgend (Suggested)', exist_ok=True)

			### --- 401 Lenguage [English]
os.makedirs(f'{root}/{a4}/01.Lenguage [English] (Suggested)/{rev}', exist_ok=True)

paths_list = ["Unit01.Comprehension", "Unit02.LiteraryAnalysis", "Unit03.WritingEssays", "Unit04.GrammarAndUsage", "Unit05.CreativeWriting", "Unit06.Speaking", "Unit07.PersuasiveWriting", "Unit08.ThecnicalVocabulary"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/01.Lenguage [English] (Suggested)/{path}', exist_ok=True)

			### --- 402 Math
os.makedirs(f'{root}/{a4}/02.Math (Suggested)/{rev}', exist_ok=True)

paths_list = ["Unit01.Algebra", "Unit02.Geometry", "Unit03.Trigonometry", "Unit04.Statistics", "Unit05.Probability", "Unit06.Functions", "Unit07.Equations&Inequalities", "Unit08.LinearAlgebra", "Unit09.Polynomials", "Unit10.Sequences&Series", "Unit11.Logarithms&Exponents", "Unit12.RationalExpressions", "Unit13.QuadraticFunctions", "Unit14.SimultaneousEquations", "Unit15.MathematicalModeling"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/02.Math (Suggested)/{path}', exist_ok=True)

			### --- 403 Technology
os.makedirs(f'{root}/{a4}/03.Teachnology (Suggested)/{rev}', exist_ok=True)

paths_list = ["Unit01.Foundations", "Unit02.OperatingSystems", "Unit03.DataStructures", "Unit04.Networks&Communications", "Unit05.Databases", "Unit06.WebDevelopment", "Unit07.Automation", "Unit08.ArtificalInteligence", "Unit09.Cybersecurity"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/03.Teachnology (Suggested)/{path}', exist_ok=True)

			### --- 404 Science
os.makedirs(f'{root}/{a4}/04.Science (Suggested)/{rev}', exist_ok=True)

paths_list = ["Unit01.IntroductionToPhysics", "Unit02.Mechanics", "Unit03.Thermodynamics", "Unit04.WavesAndOptics", "Unit05.Electrostatics", "Unit06.ElectricityAndMagnetism", "Unit07.ModernPhysics"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/04.Science (Suggested)/{path}/{rev}', exist_ok=True)

			### --- 405 History
os.makedirs(f'{root}/{a4}/05.History (Suggested)/{rev}', exist_ok=True)

paths_list = ["01.AncientCivilizations", "02.MedievalHistory", "03.RenaissanceAndReformation", "04.AgeOfExploration", "05.IndustrialRevolution", "06.WorldWarI", "07.InterwarPeriod", "08.WorldWarII", "09.ColdWarEra", "10.ModernHistory"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/05.History (Suggested)/{path}/{rev}', exist_ok=True)

			### --- 406 ForeignLanguage [Spanish]
os.makedirs(f'{root}/{a4}/06.ForeignLanguage [Spanish] (Suggested)/{rev}', exist_ok=True)

paths_list = ["Capitulo01.IntroducciónAlEspañol", "Capitulo02.Vocabulario", "Capitulo03.Gramática", "Capitulo04.ConversaciónYExpresiones", "Capitulo05.Verbos", "Capitulo06.ComprensiónAuditiva", "Capitulo07.LecturaYComprensión", "Capitulo08.Escritura", "Capitulo09.CulturaYSociedad", "Capitulo10.CineYLiteratura", "Capitulo11.Repaso(Examen)"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/06.ForeignLanguage [Spanish] (Suggested)/{path}', exist_ok=True)

			### --- 407 ForeignLanguage [French]
os.makedirs(f'{root}/{a4}/07.ForeignLanguage [French] (Suggested)/{rev}', exist_ok=True)

paths_list = ["Chapitre01.IntroductionAuFrançais", "Chapitre02.Vocabulaire", "Chapitre03.Grammaire", "Chapitre04.ConversationEtExpressions", "Chapitre05.Verbes", "Chapitre06.CompréhensionOrale", "Chapitre07.LectureEtCompréhension", "Chapitre08.Écriture", "Chapitre09.CultureEtSociété", "Chapitre10.CinémaEtLittérature", "Chapitre11.Revue(Examen)"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/07.ForeignLanguage [French] (Suggested)/{path}', exist_ok=True)

			### --- 408 EEA [VisualArts]
os.makedirs(f'{root}/{a4}/08.EEA [VisualArts] (Suggested)/{rev}', exist_ok=True)

paths_list = ["01.IntroductionToVisualArts", "02.ElementsAndPrinciplesOfDesign", "03.ArtHistory", "04.Art&Culture", "05.ColorTheory", "06.PaintingTechniques", "07.DigitalArt", "08.ContemporaryArt", "09.ArtCriticism", "10.PortfolioDevelopment", "11.ExhibitionPreparation"]
for path in paths_list:
  os.makedirs(f'{root}/{a4}/08.EEA [VisualArts] (Suggested)/{path}/{rev}', exist_ok=True)

			### --- 409	EEA [Music]
os.makedirs(f'{root}/{a4}/09.EEA [Music] (Suggested)/{rev}', exist_ok=True)

paths_list = ["01.IntroductionToMusic", "02.MusicHistory", "03.MusicTheory", "04.Rhythm&Meter", "05.Melody&Harmony", "06.Notation", "07.InstrumentalTechniques", "08.CompositionAndSongwriting", "09.EnsemblePerformance", "10.Production", "11.Demo(PortfolioExam)"]

			### --- 410	EEA [AcademicDecathlon]
			### --- 411	MyCourses

		## --- 500 HOBBIES
os.makedirs(f'{root}/{a5}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a5}/{res}', exist_ok=True)

			### --- 501 Hobbie's Name
os.makedirs(f'{root}/{a5}/01.Hobbie\'s-Name(Suggested)/{rev}', exist_ok=True)

			### --- 502 Hobbie's Name
os.makedirs(f'{root}/{a5}/02.Hobbie\'s-Name(Suggested)/{rev}', exist_ok=True)

			### --- 503 Hobbie's Name
os.makedirs(f'{root}/{a5}/03.Hobbie\'s-Name(Suggested)/{rev}', exist_ok=True)

# Crear readme.txt

readme_txt = os.path.join(root, 'ReadMe.txt')

# Crear y escribir en el archivo de texto dentro del nuevo directorio
with open(readme_txt, 'w') as file:
    file.write('this is a readme file.')


print(' ')
print('------- Successfully executed script [100%] -------')
print(' ')
print('   Please Close this window.')
print('   ANT is ready to use in: ', os.getcwd())
print('   If you need help, Read the documentation: https://github.com/MikeWorldYt/ANT-on-the-system')