/**
 * Introductory pipeline diagram (EN + ES).
 *
 * Replaces the ASCII tree previously inlined in
 * docs-{en,es}/getting-started/introduction.mdx. Intentionally a
 * coarse overview — the full architecture diagram lives in
 * compile-pipeline.ts and on the architecture/pipeline page.
 *
 * Rendered at Astro build time so the served page contains inline
 * SVG and inherits the host's CSS custom properties for theming.
 */

import { flowchart } from '@achronyme/achdiagrams';

interface Labels {
  src: string;
  parser: string;
  parserSubtitle: string;
  bc: string;
  bcSubtitle: string;
  ssa: string;
  ssaSubtitle: string;
  r1cs: string;
  r1csSubtitle: string;
  plonk: string;
  plonkSubtitle: string;
  r1csOut: string;
  r1csOutSubtitle: string;
  plonkOut: string;
  plonkOutSubtitle: string;
  proof: string;
  prove: string;
  proveSubtitle: string;
  edgeRun: string;
  edgeCircuit: string;
  edgeInline: string;
}

function build(L: Labels) {
  return flowchart()
    .node('src', { label: L.src, shape: 'data' })
    .node('parser', { label: L.parser, subtitle: L.parserSubtitle })
    .node('bc', { label: L.bc, subtitle: L.bcSubtitle })
    .node('ssa', { label: L.ssa, subtitle: L.ssaSubtitle })
    .node('r1cs', { label: L.r1cs, subtitle: L.r1csSubtitle })
    .node('plonk', { label: L.plonk, subtitle: L.plonkSubtitle })
    .node('r1csOut', { label: L.r1csOut, subtitle: L.r1csOutSubtitle, shape: 'data' })
    .node('plonkOut', { label: L.plonkOut, subtitle: L.plonkOutSubtitle, shape: 'data' })
    .node('proof', { label: L.proof, shape: 'terminator' })
    .node('prove', { label: L.prove, subtitle: L.proveSubtitle })
    .edge('src', 'parser')
    .edge('parser', 'bc', { label: L.edgeRun })
    .edge('parser', 'ssa', { label: L.edgeCircuit })
    .edge('ssa', 'r1cs')
    .edge('ssa', 'plonk')
    .edge('r1cs', 'r1csOut')
    .edge('plonk', 'plonkOut')
    .edge('r1csOut', 'proof')
    .edge('plonkOut', 'proof')
    .edge('src', 'prove', { label: L.edgeInline })
    .render();
}

const enLabels: Labels = {
  src: 'Source (.ach)',
  parser: 'Parser → AST',
  parserSubtitle: 'Pratt + recursive descent',
  bc: 'Bytecode → VM',
  bcSubtitle: 'run mode',
  ssa: 'SSA IR + Optimize',
  ssaSubtitle: 'circuit mode',
  r1cs: 'R1CS',
  r1csSubtitle: 'Groth16',
  plonk: 'Plonkish',
  plonkSubtitle: 'KZG-PlonK',
  r1csOut: '.r1cs + .wtns',
  r1csOutSubtitle: 'snarkjs compatible',
  plonkOut: 'Gates + Lookups',
  plonkOutSubtitle: 'copy constraints',
  proof: 'Native proof',
  prove: 'prove { } block',
  proveSubtitle: 'compile + witness + verify (inline)',
  edgeRun: 'ach run',
  edgeCircuit: 'ach circuit',
  edgeInline: 'inline',
};

const esLabels: Labels = {
  src: 'Fuente (.ach)',
  parser: 'Parser → AST',
  parserSubtitle: 'Pratt + descenso recursivo',
  bc: 'Bytecode → VM',
  bcSubtitle: 'modo ejecución',
  ssa: 'SSA IR + Optimizar',
  ssaSubtitle: 'modo circuito',
  r1cs: 'R1CS',
  r1csSubtitle: 'Groth16',
  plonk: 'Plonkish',
  plonkSubtitle: 'KZG-PlonK',
  r1csOut: '.r1cs + .wtns',
  r1csOutSubtitle: 'compatible con snarkjs',
  plonkOut: 'Gates + Lookups',
  plonkOutSubtitle: 'copy constraints',
  proof: 'Prueba nativa',
  prove: 'bloque prove { }',
  proveSubtitle: 'compilar + testigo + verificar (en línea)',
  edgeRun: 'ach run',
  edgeCircuit: 'ach circuit',
  edgeInline: 'en línea',
};

export const introPipelineSvgEn = build(enLabels).svg;
export const introPipelineSvgEs = build(esLabels).svg;
