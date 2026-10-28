---
title: RetinaCare knowledge graph overview
type: graph
domain: documentation
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["00-index.md"]
owner: project team
tags: [obsidian, graph]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/memory/graph-overview.md (sha256 7684ed341026); verify current state in [[active-phase]] -->
# Knowledge graph overview

Start at [[Archive/dird-memory-pre-consolidation/00-index]]. Read [[Archive/dird-memory-pre-consolidation/01-claude-start-here]] and [[Archive/dird-memory-pre-consolidation/active-phase]] before
following one workstream. Obsidian Graph View uses the real wikilinks below;
this Mermaid diagram is a readable supplement.

```mermaid
flowchart TD
  HUB[00-index] --> START[01-claude-start-here]
  HUB --> STATE[active-phase]
  HUB --> ARCH[architecture]
  HUB --> OWNER[agent-ownership]
  HUB --> PIPE[model-pipeline]
  PIPE --> S1[stage-i-screening]
  PIPE --> S2[stage-ii-segmentation]
  PIPE --> S3[stage-iii-grading]
  PIPE --> INV[model-inventory]
  S1 --> LICENSE[licensing-and-data]
  S2 --> CONTRACT[data-contracts]
  S2 --> S3
  HUB --> GUI[gui-implementation]
  GUI --> CONTRACT
  GUI --> RISK[risks-and-blockers]
  HUB --> BUILD[build-and-test]
  BUILD --> CUDA[cuda-and-packaging]
  STATE --> NEXT[next-actions]
  NEXT --> RISK
  HUB --> CHANGE[changelog-and-handoffs]
```

Validation on 2026-10-28: 32 notes, 135 resolved directed wikilink edges,
0 broken links, 0 orphan notes, and 73 existing source references. External
repository docs use relative Markdown links and do not become graph nodes.
Mermaid fences and YAML frontmatter parse; Obsidian rendering was not tested.
See [[Archive/dird-memory-pre-consolidation/README]] for opening instructions and vault-selection limits.
