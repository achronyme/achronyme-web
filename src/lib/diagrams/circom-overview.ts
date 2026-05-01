/**
 * Circom interop overview diagram (LR).
 *
 * Migrated from the Mermaid `graph LR` in docs-{en,es}/circom/overview.mdx.
 */

import { flowchart } from '@achronyme/diagrams';

interface Labels {
  circom: string;
  ach: string;
  parser: string;
  parserSubtitle: string;
  aparser: string;
  analysis: string;
  analysisSubtitle: string;
  lower: string;
  lowerSubtitle: string;
  proveir: string;
  acomp: string;
  inst: string;
  instSubtitle: string;
  ir: string;
  opt: string;
  r1cs: string;
  plonk: string;
  groth: string;
  halo: string;
  edgeProveBlock: string;
  edgeImported: string;
}

function build(L: Labels) {
  return flowchart()
    .node('circom', { label: L.circom, shape: 'data' })
    .node('ach', { label: L.ach, shape: 'data' })
    .node('parser', { label: L.parser, subtitle: L.parserSubtitle })
    .node('aparser', { label: L.aparser })
    .node('analysis', { label: L.analysis, subtitle: L.analysisSubtitle })
    .node('lower', { label: L.lower, subtitle: L.lowerSubtitle })
    .node('proveir', { label: L.proveir })
    .node('acomp', { label: L.acomp })
    .node('inst', { label: L.inst, subtitle: L.instSubtitle })
    .node('ir', { label: L.ir })
    .node('opt', { label: L.opt })
    .node('r1cs', { label: L.r1cs })
    .node('plonk', { label: L.plonk })
    .node('groth', { label: L.groth })
    .node('halo', { label: L.halo })
    .edge('circom', 'parser')
    .edge('ach', 'aparser')
    .edge('parser', 'analysis')
    .edge('analysis', 'lower')
    .edge('lower', 'proveir')
    .edge('aparser', 'acomp')
    .edge('acomp', 'proveir', { label: L.edgeProveBlock })
    .edge('acomp', 'proveir', { label: L.edgeImported })
    .edge('proveir', 'inst')
    .edge('inst', 'ir')
    .edge('ir', 'opt')
    .edge('opt', 'r1cs')
    .edge('opt', 'plonk')
    .edge('r1cs', 'groth')
    .edge('plonk', 'halo')
    .render({ direction: 'LR' });
}

const enLabels: Labels = {
  circom: '.circom source',
  ach: '.ach source',
  parser: 'Circom Parser',
  parserSubtitle: 'circom/src/parser',
  aparser: 'Achronyme Parser',
  analysis: 'Constraint Analysis',
  analysisSubtitle: 'E100-E102, W101-W103',
  lower: 'Lowering',
  lowerSubtitle: 'signals, expressions, statements',
  proveir: 'ProveIR',
  acomp: 'Achronyme Compiler',
  inst: 'Instantiate',
  instSubtitle: 'capture scope values',
  ir: 'SSA IR',
  opt: 'Optimize',
  r1cs: 'R1CS',
  plonk: 'Plonkish',
  groth: 'Groth16',
  halo: 'PlonK (halo2)',
  edgeProveBlock: 'prove { } block',
  edgeImported: 'imported templates',
};

const esLabels: Labels = {
  circom: 'fuente .circom',
  ach: 'fuente .ach',
  parser: 'Parser Circom',
  parserSubtitle: 'circom/src/parser',
  aparser: 'Parser Achronyme',
  analysis: 'Análisis de Constraints',
  analysisSubtitle: 'E100-E102, W101-W103',
  lower: 'Lowering',
  lowerSubtitle: 'signals, expresiones, statements',
  proveir: 'ProveIR',
  acomp: 'Compilador Achronyme',
  inst: 'Instanciar',
  instSubtitle: 'capturar valores del scope',
  ir: 'SSA IR',
  opt: 'Optimizar',
  r1cs: 'R1CS',
  plonk: 'Plonkish',
  groth: 'Groth16',
  halo: 'PlonK (halo2)',
  edgeProveBlock: 'bloque prove { }',
  edgeImported: 'plantillas importadas',
};

export const circomOverviewSvgEn = build(enLabels).svg;
export const circomOverviewSvgEs = build(esLabels).svg;
