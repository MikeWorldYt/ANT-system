#!/usr/bin/env python3
# modify dev 
import os
import re
from datetime import datetime
import time

def add_prefix(path='.'):
    script_name = ignore_scrypt()
    # Primera pasada: Agrega solo el prefijo ~ en el indice 0
    for fname in os.listdir(path):
        if fname == script_name:
            continue
        src = os.path.join(path, fname)
        if not os.path.isfile(src) or fname.startswith('~'):
            continue
        new_name = f"~{fname}"
        dst = os.path.join(path, new_name)
        print(f"Adding prefix: {fname} -> {new_name}")
        os.rename(src, dst)
    print("Prefixing done.\n")

def add_date(path='.'):
    script_name = ignore_scrypt()
    # Segunda pasada: Formatear fecha si se necesita YY-MM-DD
    date_str = datetime.now().strftime("%y-%m-%d")
    # Patron para detectar si ya existe una fecha entre parentesis como (27-01-05) o (27-01)
    date_pattern = re.compile(r'\(\d{2}-\d{2}(?:-\d{2})?\)')

    # iteration para agregar fecha
    for fname in os.listdir(path):
        if fname == script_name:
            continue
        src = os.path.join(path, fname)
        print("3...")
        time.sleep(2)
        # Ignoramos si el archivo ya tiene fecha
        if not os.path.isfile(src):
            continue
        if date_pattern.search(fname):
            continue
        new_name = f"~({date_str}){fname[1:]}"  # Inserta fecha despues del indice 1
        dst = os.path.join(path, new_name)
        
        os.rename(src, dst)

    print("Done DEF.")
    time.sleep(50)

# Funcion ignorar el propio scrypt
def ignore_scrypt(path='.'):
    return os.path.basename(__file__)

# PASOS
if __name__ == '__main__':
    print("Step 1: Prefixing all files with ~")
    add_prefix()
    time.sleep(1)
    ans = input("Do you need date? (y/n): ").strip().lower()
    if ans == 'y':
        print("Step 2: Adding date to prefixed files")
        add_date()
    print("Done.")
    time.sleep(10)
