import os

# Make Dir of Level 1
root = 'Root_Storage'
a1 = '001.PERSONAL'
a2 = '002.WORK'
a3 = '003.PROJECTS'
a4 = '004.ACADEMIC'
a5 = '005.HOBBIES'
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
os.makedirs(f'{root}/{a1}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/{res}', exist_ok=True)

## --- 01 personal
os.makedirs(f'{root}/{a1}/01.MyDocuments(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/01.MyDocuments(Suggested)/01.FinancialDocs(Suggested)', exist_ok=True)
os.makedirs(f'{root}/{a1}/02.FamilyDocuments(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/03.OtherDocuments(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a1}/04.MyGallery(Suggested)/01.Family(Suggested)', exist_ok=True)
os.makedirs(f'{root}/{a1}/05.Portfolio(Suggested)/{rev}', exist_ok=True)

## --- 02 work
os.makedirs(f'{root}/{a2}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a2}/{res}', exist_ok=True)

os.makedirs(f'{root}/{a2}/01.Category(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a2}/02.Category(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a2}/03.Category(Suggested)/{rev}', exist_ok=True)

## --- 03 projects
os.makedirs(f'{root}/{a3}/{rev}', exist_ok=True)

## --- 04 academic
os.makedirs(f'{root}/{a4}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a4}/{res}', exist_ok=True)

os.makedirs(f'{root}/{a4}/01.Subjet-Names(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a4}/02.Subjet-Names(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a4}/03.Subjet-Names(Suggested)/{rev}', exist_ok=True)

## --- 05 hobbies
os.makedirs(f'{root}/{a5}/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a5}/{res}', exist_ok=True)

os.makedirs(f'{root}/{a5}/01.Hobbie\'s-Name(Suggested)/{rev}', exist_ok=True)
os.makedirs(f'{root}/{a5}/02.Hobbie\'s-Name(Suggested)/{rev}', exist_ok=True)
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