import os

# Make Dir of Level 1
os.makedirs('Root_Storage', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL', exist_ok=True)
os.makedirs('Root_Storage/002.WORK', exist_ok=True)
os.makedirs('Root_Storage/003.PROJECTS', exist_ok=True)
os.makedirs('Root_Storage/004.ACADEMIC', exist_ok=True)
os.makedirs('Root_Storage/005.HOBBIES', exist_ok=True)
os.makedirs('Root_Storage/900.Resources', exist_ok=True)
os.makedirs('Root_Storage/998.Review', exist_ok=True)
os.makedirs('Root_Storage/999.Setting', exist_ok=True)

# Make Dir of Level 2 and 3
os.makedirs('Root_Storage/001.PERSONAL/98.Review', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/97.Undefined', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/90.Resources', exist_ok=True)

os.makedirs('Root_Storage/001.PERSONAL/01.MyDocuments(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/01.MyDocuments(Suggested)/01.FinancialDocs(Suggested)', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/02.FamilyDocuments(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/03.OtherDocuments(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/04.MyGallery(Suggested)/01.Family(Suggested)', exist_ok=True)
os.makedirs('Root_Storage/001.PERSONAL/05.Portfolio(Suggested)/98.Review', exist_ok=True)

## --- 02
os.makedirs('Root_Storage/002.WORK/98.Review', exist_ok=True)
os.makedirs('Root_Storage/002.WORK/97.Undefined', exist_ok=True)
os.makedirs('Root_Storage/002.WORK/90.Resources', exist_ok=True)

os.makedirs('Root_Storage/002.WORK/01.Category(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/002.WORK/02.Category(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/002.WORK/03.Category(Suggested)/98.Review', exist_ok=True)

## --- 03
os.makedirs('Root_Storage/003.PROJECTS/98.Review', exist_ok=True)
os.makedirs('Root_Storage/003.PROJECTS/97.Undefined', exist_ok=True)

## --- 04
os.makedirs('Root_Storage/004.ACADAMIC/98.Review', exist_ok=True)
os.makedirs('Root_Storage/004.ACADAMIC/97.Undefined', exist_ok=True)
os.makedirs('Root_Storage/004.ACADAMIC/90.Resources', exist_ok=True)

os.makedirs('Root_Storage/004.ACADEMIC/01.Subjet-Names(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/004.ACADEMIC/02.Subjet-Names(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/004.ACADEMIC/03.Subjet-Names(Suggested)/98.Review', exist_ok=True)

## --- 05
os.makedirs('Root_Storage/005.HOBBIES/98.Review', exist_ok=True)
os.makedirs('Root_Storage/005.HOBBIES/97.Undefined', exist_ok=True)
os.makedirs('Root_Storage/005.HOBBIES/90.Resources', exist_ok=True)

os.makedirs('Root_Storage/005.HOBBIES/01.Hobbie\'s-Name(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/005.HOBBIES/02.Hobbie\'s-Name(Suggested)/98.Review', exist_ok=True)
os.makedirs('Root_Storage/005.HOBBIES/03.Hobbie\'s-Name(Suggested)/98.Review', exist_ok=True)

print(' ')
print('------- Successfully executed script [100%] -------')
print(' ')
print('   Please Close this window.')
print('   If you need help, Read the documentation: https://github.com/MikeWorldYt/ANT-on-the-system')