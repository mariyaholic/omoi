# Open Design references used for Omoi

Reviewed on 2026-11-03 from the installed application's local data, not a web search:

`C:\Users\<you>\AppData\Roaming\Open Design\namespaces\release-stable-win\data\design-systems`

Five packages are present. I read their design/provenance notes and inspected static `components.html` renders for all five. Also inspected the app-kit sidebar/list source for Confluence and Panel. The reference renders disabled JavaScript and blocked HTTP requests; custom web fonts were not downloaded, so the renders can use local fallbacks. Source notes remain the evidence for named typefaces. The source packages were not edited.

| System | Distinctive sourced traits | Application to Omoi |
|---|---|---|
| **Uika** | Violet-black #0b0913, warm platinum #e9e4da, gold #bb9955; Cinzel/Outfit/JetBrains roles; small radii | Keep the default Night palette and restrained gold active state. Omoi's reading headings use local Georgia, explicitly a design choice rather than a claim to reproduce Cinzel. |
| **Mariyaholic Panel** | Charcoal #121418 and #1a1d23, gold #d4b372, Segoe UI, compact utility controls, small corners, no animation/shadows specified in source | Add a selectable Panel appearance with native utility typography, compact rows, stronger functional hierarchy, and no panel shadow. Use system fonts without redistributing them. |
| **Mariyaholic Confluence** | Paper #f3efe4, wells #e9e2d5/#ded4c4, ink #231f19, brick #94301f, serif reading hierarchy, square edges | Rework Paper appearance around these colors, square controls, and an italic editorial treatment on rediscovery titles. Georgia is the offline serif fallback, not a bundled Fraunces/Newsreader copy. |
| **Mariyaholic-98** | 16-color indigo/coral palette, Departure Mono on an 11px heading grid, square windows and hard offset shadows | A dedicated PC-98 appearance: source palette, local Departure Mono/JetBrains/Consolas stack, 44/33/22px headings, square controls, hard shadows, dashed focus and dithered navigation chrome. Keep reading surfaces solid. Semantic foregrounds use the brighter source palette for legibility. |
| **Argonaut Oak** | Pine-black/gold, semantic state colors, dense information hierarchy and explicit mono roles | A dedicated Oak appearance: pine panels, brass selection, cyan timestamp metadata, sage/clay status text, compact rows and Outfit/Inter/Segoe UI local stacks. Retain readable body sizes instead of copying the terminal's smallest type. |

## Important distinction

The packages mark values **sourced**, **derived** or **default**. Many layout widths, spacing steps, shadows, type scales and component examples are OpenDesign importer defaults. The app-kit JSX includes generic assistant-workspace layouts and CDN React/Babel bootstrapping. These are scaffolds, not evidence of an approved Omoi layout or deployment stack. The prototype remains plain local HTML/CSS/JS and makes no external requests.

`inspiration.css` contains the scoped optional studies. It deliberately leaves `tokens.css` and the estate canon untouched. Values absent from the source palette (strong-line variants, emphasis shades and spacing) are Omoi-derived choices, not promoted to estate canon. Uika remains the initial appearance. **Settings → Appearance** now provides five source-labelled choices: **Uika**, **Mariyaholic Confluence**, **Mariyaholic Panel**, **Mariyaholic-98**, and **Argonaut Oak**. Each includes palette swatches, a short description and a visible selected state. The same library, actions and navigation work in every appearance.

All font stacks are local-only. Named source fonts are used when installed, otherwise the browser uses the documented system fallbacks; no font binaries or external services are required. These are Omoi adaptations of each design system, not pixel-exact reproductions of the original applications.

## Local sources

- [Uika DESIGN.md](<C:/Users/you/AppData/Roaming/Open Design/namespaces/release-stable-win/data/design-systems/uika/DESIGN.md>)
- [Mariyaholic Panel DESIGN.md](<C:/Users/you/AppData/Roaming/Open Design/namespaces/release-stable-win/data/design-systems/mariyaholic-panel/DESIGN.md>)
- [Confluence DESIGN.md](<C:/Users/you/AppData/Roaming/Open Design/namespaces/release-stable-win/data/design-systems/mariyaholic-confluence/DESIGN.md>)
- [Mariyaholic-98 DESIGN.md](<C:/Users/you/AppData/Roaming/Open Design/namespaces/release-stable-win/data/design-systems/mariyaholic-98/DESIGN.md>)
- [Argonaut Oak DESIGN.md](<C:/Users/you/AppData/Roaming/Open Design/namespaces/release-stable-win/data/design-systems/argonaut-oak/DESIGN.md>)

Reference screenshots are retained at `D:\mariyaholic-build\omoi\opendesign-review`. The latest five-theme prototype evidence is separate at `C:\Users\<you>\.codex\visualizations\2026\10\27\01a17f2f-e6fe-7ac3-ba10-e43e7f5af853\omoi-five-themes`.
