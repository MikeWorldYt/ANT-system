# Dictionary

`<folder>/.ant/dictionary.md` groups the files of review folders at or below that folder. A closer dictionary adds to the farther ones and replaces their entries of the same name. Read with `get_dictionary`, write with `set_dictionary` (it replaces the whole file; send every entry back).

Each entry is a `## Heading` naming where its files go, refined by keys:
- `term`: matches and is removed from the name.
- `topics`: match and stay once in the name, in Train-Case and moved right after the date and owner; repeats leave. Other forms go in parentheses and are rewritten to the topic: `Sakuya (Izayoi Sakuya, Sakuya Izayoi, Iza)`.
- `only`: matches case and symbols exactly, as a whole word, and stays.
- `exclude`: only removed from the name; never decides the group.
- `ext`: extensions, e.g. `.exe .msi`.
- `pattern`: a case-insensitive regex; its match is removed. One per line.
- `prefix` / `suffix`: text added at the start / end of each grouped name (the suffix before the extension and any " (2)" counter), once. Above the first entry they reach every file of the folder; an entry's own wins.

Headings:
- `## Name`: a subfolder of the review folder, created when needed.
- `## Do not group`: files stay where they are but are still renamed by the entry.
- `## NODE.Title`: a node beside the review folder (a child of the dictionary's folder), matched by title without its id.
- `## ./*.Art/*.Touhou`: a path from the dictionary's folder, each segment by title or with `*` for any id; every `../` before it climbs one level.
- `## NODE^.Title`: one level up, a sibling of the dictionary's folder. A path walks down: `NODE^.Blue-Archive/Fanarts`.
- A node that takes files keeps them; any other passes them to its own review folder. Renaming a node in the app updates its NODE headings.

To see what a dictionary does before anything moves, run `plan_automations` on the folder and show the user the result (see ant://guide/automations).

The longest matching term, topic or only across entries wins; `ext` and `pattern` decide only when no word matched. `%% ... %%` blocks are comments.
