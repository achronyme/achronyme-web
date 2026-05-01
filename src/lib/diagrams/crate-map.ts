/**
 * Achronyme crate dependency graph (TB).
 *
 * Migrated from the ASCII art in docs-{en,es}/architecture/crate-map.mdx.
 *
 * Each edge represents "depends on": a crate at a lower layer is built
 * upon by a crate at a higher one. Cross-tier annotations from the ASCII
 * (`uses lysis-types`, `uses artik, lysis-types`) become explicit edges
 * here so the layered layout can place them.
 *
 * Crate names don't translate; only the section header and the
 * tier-leaf annotations differ between EN and ES.
 */

import { flowchart } from '@achronyme/diagrams';

interface Labels {
  tier0: string;
  cliUsesEverything: string;
}

function build(L: Labels) {
  return flowchart()
    // Tier 0 — pure leaves
    .node('diagnostics', { label: 'diagnostics', subtitle: L.tier0 })
    .node('lysisTypes', { label: 'lysis-types', subtitle: L.tier0 })
    .node('achMacros', { label: 'ach-macros', subtitle: L.tier0 })

    // Tier 1 — parser + memory
    .node('parser', { label: 'achronyme-parser' })
    .node('memory', { label: 'memory' })

    // Tier 2 — resolve, constraints
    .node('resolve', { label: 'resolve' })
    .node('constraints', { label: 'constraints' })

    // Tier 3 — ir-core
    .node('irCore', { label: 'ir-core' })

    // Tier 4 — ir-forge
    .node('irForge', { label: 'ir-forge' })

    // Tier 5 — ir
    .node('ir', { label: 'ir' })

    // Tier 6 — backend + frontend crates
    .node('zkc', { label: 'zkc' })
    .node('circom', { label: 'circom' })
    .node('akronc', { label: 'akronc' })
    .node('akron', { label: 'akron' })
    .node('artik', { label: 'artik' })

    // Tier 7 — lysis (uses artik + lysis-types)
    .node('lysis', { label: 'lysis' })

    // Tier 8 — root layer
    .node('proving', { label: 'proving' })
    .node('std', { label: 'achronyme-std' })
    .node('cli', { label: 'cli', subtitle: L.cliUsesEverything, shape: 'terminator' })

    // Tier 0 → Tier 1
    .edge('diagnostics', 'parser')
    .edge('diagnostics', 'memory')

    // Tier 1 → Tier 2
    .edge('parser', 'resolve')
    .edge('memory', 'constraints')

    // Tier 2 → Tier 3
    .edge('memory', 'irCore')
    .edge('constraints', 'irCore')

    // Tier 3 + Tier 0 (lysis-types) → Tier 4
    .edge('irCore', 'irForge')
    .edge('lysisTypes', 'irForge')

    // Tier 4 → Tier 5
    .edge('irForge', 'ir')

    // Tier 5 → Tier 6
    .edge('ir', 'zkc')
    .edge('ir', 'circom')
    .edge('ir', 'akronc')
    .edge('ir', 'akron')
    .edge('ir', 'artik')

    // Tier 6 + Tier 0 → Tier 7
    .edge('artik', 'lysis')
    .edge('lysisTypes', 'lysis')

    // Tier 7 → Tier 8
    .edge('lysis', 'proving')
    .edge('lysis', 'std')
    .edge('lysis', 'cli')
    .edge('zkc', 'cli')
    .edge('circom', 'cli')
    .edge('akronc', 'cli')
    .edge('akron', 'cli')

    // ach-macros stitches in via parser
    .edge('achMacros', 'parser')

    .render();
}

const enLabels: Labels = {
  tier0: 'Tier 0 leaf',
  cliUsesEverything: 'uses everything',
};

const esLabels: Labels = {
  tier0: 'leaf de Tier 0',
  cliUsesEverything: 'usa todo',
};

export const crateMapSvgEn = build(enLabels).svg;
export const crateMapSvgEs = build(esLabels).svg;
