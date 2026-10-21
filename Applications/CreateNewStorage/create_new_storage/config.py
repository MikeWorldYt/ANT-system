# Where the wizard downloads the storage template, guide and tools from.
import os

from ant_core.version import read_app_version


APP_DIR        = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ANT_VERSION    = read_app_version(APP_DIR)
DOCS_URL       = "https://ant-system.vercel.app/en/start-here/migrating-plan"
README_URL     = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/01.Get_Started.md"
README_NAME    = "01.Get_Started.md"
TREE_YAML_URL  = "https://raw.githubusercontent.com/MikeWorldYt/ANT-system/main/Migration/Core/Tree.yaml"
TREE_YAML_NAME = "Tree.yaml"
CORE_API_URL        = "https://api.github.com/repos/MikeWorldYt/ANT-system/contents/Migration/Core"
FOLDER_MANAGER_URL  = "https://github.com/MikeWorldYt/ANT-system/raw/main/Applications/FolderManager/dist/FolderManager.exe"
FOLDER_MANAGER_NAME = "FolderManager.exe"
