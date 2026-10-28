<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/plugins-deck-and-mcp-pipeline.md (sha256 6ed07e0ef6ae); verify current state in [[active-phase]] -->
# Plugins Deck and MCP Pipeline

## Interface Architecture
The Plugins Deck is located at AppPage::Plugins matching the reference deck layout.
Exclusively displays plugins and connectors with skills deck excluded per user instruction.
Includes header Customize title, filter toggle, and Add dropdown action.
Top controls feature category pills, sub-filter toggles All, Yours, Discover, and live search.
Curated hero banner features deep forest green card, carousel controls, and app badges.
Catalog displays Top connectors in a responsive 2-column card grid with verified badges.

## Authentic 12 Plugin Connectors
Google Drive: Official 3-color ribbon on dark charcoal tile with accurate geometry and overlap shading.
Gmail: Official Google 4-color envelope M on dark tile with proper brand scale.
Google Calendar: Official 4-color framed square with 31 glyph on dark tile.
Microsoft 365: Official 4 vibrant rounded squares (orange-red, green, cyan, yellow).
Slack: Official 4-color octothorpe with capsules and dots on dark tile.
Notion: Official white 3D cube with bold black serif N on dark tile.
Canva: Official turquoise-to-purple gradient rounded square tile with cursive white C.
Atlassian MCP: Official royal blue dual curved sails on dark tile.
Google Sheets: Official green folded document with white 2x3 spreadsheet grid.
Figma: Official 5-color stacked shapes (orange, pink, purple, cyan, green).
Google Docs: Official royal blue folded document with horizontal document lines.
Supabase: Official emerald green faceted lightning bolt on dark tile.
Hero banner badges: Real science chips for Mendeley, bioRxiv, NIH, RSC/Tree, ChemDraw, Wiley, and Books.
Assets: Embedded at compile time as 128x128 PNGs with zero network latency and pure native rendering.

## Pipeline and Runtime Execution
RetinaCare uses zero WebView; all plugin communication runs via native Rust MCP clients.
Plugins expose JSON-RPC / stdio tool definitions and structured resource schemas.
Clicking the action button toggles connection state instantly between Available and Connected.
Active connected plugins register into the Home Chat clinical ingestion pipeline.
When prompts are submitted, the assistant actively acknowledges operational MCP connectors.
Search query filters the 12 plugins in real time across name, author, and description.
Sub-filter Yours isolates connected plugins; Discover isolates uninstalled connectors.
Transient toast bar confirms connection and disconnection state updates.
