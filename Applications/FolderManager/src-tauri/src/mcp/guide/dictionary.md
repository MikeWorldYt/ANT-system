# Dictionary

`<folder>/.ant/dictionary.md` groups the files of review folders at or below that folder. A closer dictionary adds to the farther ones and replaces their entries of the same name. Read with `get_dictionary`, write with `set_dictionary` (it replaces the whole file; send every entry back).

Each entry is a `## Heading` naming where its files go, refined by keys:
- `term`: matches and is removed from the name.
- `topics`: match and stay once in the name; repeats leave, multi-word topics split into words.
- `only`: matches case and symbols exactly, as a whole word, and stays.
- `exclude`: only removed from the name; never decides the group.
- `ext`: extensions, e.g. `.exe .msi`.
- `pattern`: a case-insensitive regex; its match is removed. One per line.

Headings:
- `## Name`: a subfolder of the review folder, created when needed.
- `## Do not group`: files stay where they are but are still renamed by the entry.
- `## NODE.Title`: a node beside the review folder (a child of the dictionary's folder), matched by title without its id.
- `## NODE^.Title`: one level up, a sibling of the dictionary's folder. A path walks down: `NODE^.Blue-Archive/Fanarts`.
- A node that takes files keeps them; any other passes them to its own review folder. Renaming a node in the app updates its NODE headings.

The longest matching term, topic or only across entries wins; `ext` and `pattern` decide only when no word matched. `%% ... %%` blocks are comments.
