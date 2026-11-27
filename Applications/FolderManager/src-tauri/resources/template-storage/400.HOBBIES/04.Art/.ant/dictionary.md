%%
See the documentation to build the entries correctly:
https://ant.mikeworld.dev/en/advanced/automations#dictionaries/
%%

# Dictionary

## NODE^.Blue-Archive/Fanarts
- term: Blue Archive, BlueArchive, BA
- topics: Yuuka, Asuna Ichinose, Asuna, Noa Ushio, Mika Misono, Nonomi
- exclude: sensei

## NODE.Bocchi-the-Rock
- term: BocchiTR, Bocchi the Rock, Bocchi
- topics: Rio Yamada

## Genshin-Impact
- term: Genshin Impact, Genshin, GI
- topics: Ganyu, Qiqi, Keqing, Faruzan

## Touhou
- term: Touhou, 
- topics: Sakuya Izayoi (Izayoi Sakuya)
- only: Twilight Frontier, Twilight-Frontier

## Demon-Slayer
- term: Demon Slayer, Kimetsu No Yaiba

## Pokemon 
- term: Pokemon
- topics: Meowscarada

## One-Piece 
- term: One piece
- topics: Nico Robin

## Wplace
- term: wplace
- ext: .wplace

## AI-generated
- term: Gemini Generated, pixai
- only: ChatGPT Image

## Screenshots
- only: Screenshot

## WhatsApp
- only: whatsapp, Whatsapp
- pattern: ^(IMG|VID|AUD|PTT)-\d{8}-WA\d+

# Executables
- only: Setup, Installer, Install, Portable, Launcher
- ext: .exe .msi

## Do not group
- term: wallhaven, wallpaper
- topics: Fanart, Cosplay, Anime, Manga, Waifu, Pixiv, drawn by, Konachan, yande
- pattern: ^FB_IMG_
- pattern: ^PXL_\d{8}_
