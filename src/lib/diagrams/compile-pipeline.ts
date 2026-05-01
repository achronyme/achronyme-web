/**
 * Achronyme compile pipeline diagram (EN + ES).
 *
 * Migrated from the Mermaid `graph TD` that previously lived inline in
 * the architecture/pipeline.mdx files. Renders at Astro build time so
 * the served pages contain inline SVG and inherit the host's CSS
 * custom properties for theming.
 */

import { flowchart } from '@achronyme/achdiagrams';

interface Labels {
  src: string;
  circomSrc: string;
  parser: string;
  parserSubtitle: string;
  circomParser: string;
  circomParserSubtitle: string;
  ast: string;
  circomAst: string;
  analysis: string;
  analysisSubtitle: string;
  circomLower: string;
  circomLowerSubtitle: string;
  proveirCircom: string;
  bc: string;
  optBc: string;
  optBcSubtitle: string;
  vm: string;
  vmSubtitle: string;
  proveir: string;
  proveirSubtitle: string;
  bytes: string;
  bytesSubtitle: string;
  inst: string;
  instSubtitle: string;
  lower: string;
  ir: string;
  opt: string;
  optSubtitle: string;
  taint: string;
  taintSubtitle: string;
  bool: string;
  boolSubtitle: string;
  r1cs: string;
  plonk: string;
  groth: string;
  grothSubtitle: string;
  exportNode: string;
  exportSubtitle: string;
  halo: string;
  haloSubtitle: string;
  proof: string;
  sol: string;
  solSubtitle: string;
  edgeProveBlock: string;
  edgeAchCircom: string;
  edgeAchCircuit: string;
}

function build(L: Labels) {
  return flowchart()
    .node('src', { label: L.src, shape: 'data' })
    .node('circomSrc', { label: L.circomSrc, shape: 'data' })
    .node('parser', { label: L.parser, subtitle: L.parserSubtitle })
    .node('circomParser', { label: L.circomParser, subtitle: L.circomParserSubtitle })
    .node('ast', { label: L.ast })
    .node('circomAst', { label: L.circomAst })
    .node('analysis', { label: L.analysis, subtitle: L.analysisSubtitle })
    .node('circomLower', { label: L.circomLower, subtitle: L.circomLowerSubtitle })
    .node('proveirCircom', { label: L.proveirCircom })
    .node('bc', { label: L.bc })
    .node('optBc', { label: L.optBc, subtitle: L.optBcSubtitle })
    .node('vm', { label: L.vm, subtitle: L.vmSubtitle })
    .node('proveir', { label: L.proveir, subtitle: L.proveirSubtitle })
    .node('bytes', { label: L.bytes, subtitle: L.bytesSubtitle })
    .node('inst', { label: L.inst, subtitle: L.instSubtitle })
    .node('lower', { label: L.lower })
    .node('ir', { label: L.ir })
    .node('opt', { label: L.opt, subtitle: L.optSubtitle })
    .node('taint', { label: L.taint, subtitle: L.taintSubtitle })
    .node('bool', { label: L.bool, subtitle: L.boolSubtitle })
    .node('r1cs', { label: L.r1cs })
    .node('plonk', { label: L.plonk })
    .node('groth', { label: L.groth, subtitle: L.grothSubtitle })
    .node('exportNode', { label: L.exportNode, subtitle: L.exportSubtitle, shape: 'data' })
    .node('halo', { label: L.halo, subtitle: L.haloSubtitle })
    .node('proof', { label: L.proof, shape: 'terminator' })
    .node('sol', { label: L.sol, subtitle: L.solSubtitle, shape: 'data' })
    .edge('src', 'parser')
    .edge('parser', 'ast')
    .edge('circomSrc', 'circomParser')
    .edge('circomParser', 'circomAst')
    .edge('circomAst', 'analysis')
    .edge('analysis', 'circomLower')
    .edge('circomLower', 'proveirCircom')
    .edge('ast', 'bc')
    .edge('bc', 'optBc')
    .edge('optBc', 'vm')
    .edge('ast', 'proveir')
    .edge('proveir', 'bytes')
    .edge('bytes', 'vm')
    .edge('vm', 'inst', { label: L.edgeProveBlock })
    .edge('proveirCircom', 'inst', { label: L.edgeAchCircom })
    .edge('inst', 'ir')
    .edge('ast', 'lower', { label: L.edgeAchCircuit })
    .edge('lower', 'ir')
    .edge('ir', 'opt')
    .edge('opt', 'taint')
    .edge('taint', 'bool')
    .edge('bool', 'r1cs')
    .edge('bool', 'plonk')
    .edge('r1cs', 'groth')
    .edge('r1cs', 'exportNode')
    .edge('plonk', 'halo')
    .edge('groth', 'proof')
    .edge('halo', 'proof')
    .edge('proof', 'sol')
    .render();
}

const enLabels: Labels = {
  src: 'Source (.ach)',
  circomSrc: 'Source (.circom)',
  parser: 'Parser',
  parserSubtitle: 'Pratt + recursive descent',
  circomParser: 'Circom Parser',
  circomParserSubtitle: 'hand-written Pratt',
  ast: 'AST',
  circomAst: 'Circom AST',
  analysis: 'Constraint Analysis',
  analysisSubtitle: '<-- without === = error',
  circomLower: 'Circom Lowering',
  circomLowerSubtitle: 'signals · expressions · statements',
  proveirCircom: 'ProveIR',
  bc: 'Bytecode Compiler',
  optBc: 'Bytecode Optimizer',
  optBcSubtitle: 'peephole · const fold · dead store',
  vm: 'Akron VM',
  vmSubtitle: 'register-based, 43 opcodes',
  proveir: 'ProveIR Compiler',
  proveirSubtitle: 'pre-compiled circuit templates',
  bytes: 'Serialized bytes',
  bytesSubtitle: 'constant pool TAG_BYTES',
  inst: 'ProveIR Instantiate',
  instSubtitle: 'capture scope values',
  lower: 'IR Lowering',
  ir: 'SSA IR',
  opt: 'Optimize',
  optSubtitle: 'const fold · bound inference · CSE · DCE',
  taint: 'Taint Analysis',
  taintSubtitle: 'under-constrained warnings',
  bool: 'Bool Propagation',
  boolSubtitle: 'skip redundant enforcement',
  r1cs: 'R1CS Backend',
  plonk: 'Plonkish Backend',
  groth: 'Groth16',
  grothSubtitle: 'ark-groth16 + ark-bn254',
  exportNode: '.r1cs + .wtns',
  exportSubtitle: 'snarkjs compatible',
  halo: 'PlonK',
  haloSubtitle: 'halo2 KZG (PSE fork)',
  proof: 'Proof',
  sol: 'Solidity verifier',
  solSubtitle: 'on-chain verification',
  edgeProveBlock: 'prove { } block',
  edgeAchCircom: 'ach circom',
  edgeAchCircuit: 'ach circuit',
};

const esLabels: Labels = {
  src: 'Fuente (.ach)',
  circomSrc: 'Fuente (.circom)',
  parser: 'Parser',
  parserSubtitle: 'Pratt + descenso recursivo',
  circomParser: 'Parser Circom',
  circomParserSubtitle: 'Pratt escrito a mano',
  ast: 'AST',
  circomAst: 'AST Circom',
  analysis: 'Análisis de Constraints',
  analysisSubtitle: '<-- sin === = error',
  circomLower: 'Lowering Circom',
  circomLowerSubtitle: 'signals · expresiones · statements',
  proveirCircom: 'ProveIR',
  bc: 'Compilador de Bytecode',
  optBc: 'Optimizador de Bytecode',
  optBcSubtitle: 'peephole · const fold · dead store',
  vm: 'Akron VM',
  vmSubtitle: 'basada en registros, 43 opcodes',
  proveir: 'Compilador ProveIR',
  proveirSubtitle: 'plantillas de circuito pre-compiladas',
  bytes: 'Bytes serializados',
  bytesSubtitle: 'constant pool TAG_BYTES',
  inst: 'ProveIR Instanciar',
  instSubtitle: 'capturar valores del scope',
  lower: 'Bajada a IR',
  ir: 'SSA IR',
  opt: 'Optimizar',
  optSubtitle: 'const fold · bound inference · CSE · DCE',
  taint: 'Análisis de Taint',
  taintSubtitle: 'warnings sub-restringidos',
  bool: 'Propagación Booleana',
  boolSubtitle: 'omitir enforcement redundante',
  r1cs: 'Backend R1CS',
  plonk: 'Backend Plonkish',
  groth: 'Groth16',
  grothSubtitle: 'ark-groth16 + ark-bn254',
  exportNode: '.r1cs + .wtns',
  exportSubtitle: 'compatible con snarkjs',
  halo: 'PlonK',
  haloSubtitle: 'halo2 KZG (fork PSE)',
  proof: 'Prueba',
  sol: 'Verificador Solidity',
  solSubtitle: 'verificación on-chain',
  edgeProveBlock: 'bloque prove { }',
  edgeAchCircom: 'ach circom',
  edgeAchCircuit: 'ach circuit',
};

export const compilePipelineSvgEn = build(enLabels).svg;
export const compilePipelineSvgEs = build(esLabels).svg;
export const compilePipelineSvg = compilePipelineSvgEn;
