/**
 * Lysis VM architecture diagram (TB).
 *
 * Migrated from the ASCII art in docs-{en,es}/architecture/lysis-vm.mdx.
 */

import { flowchart } from '@achronyme/achdiagrams';

interface Labels {
  proveir: string;
  walker: string;
  walkerSubtitle: string;
  bytecode: string;
  bytecodeSubtitle: string;
  exec: string;
  execSubtitle: string;
  sink: string;
  sinkSubtitle: string;
  ssa: string;
  ssaSubtitle: string;
  backend: string;
}

function build(L: Labels) {
  return flowchart()
    .node('proveir', { label: L.proveir, shape: 'terminator' })
    .node('walker', { label: L.walker, subtitle: L.walkerSubtitle })
    .node('bytecode', { label: L.bytecode, subtitle: L.bytecodeSubtitle, shape: 'data' })
    .node('exec', { label: L.exec, subtitle: L.execSubtitle })
    .node('sink', { label: L.sink, subtitle: L.sinkSubtitle })
    .node('ssa', { label: L.ssa, subtitle: L.ssaSubtitle, shape: 'data' })
    .node('backend', { label: L.backend, shape: 'terminator' })
    .edge('proveir', 'walker')
    .edge('walker', 'bytecode')
    .edge('bytecode', 'exec')
    .edge('exec', 'sink')
    .edge('sink', 'ssa')
    .edge('ssa', 'backend')
    .render();
}

const enLabels: Labels = {
  proveir: 'ProveIR',
  walker: 'lysis_lift::walker',
  walkerSubtitle: 'BTA · extract · diff · symbolic',
  bytecode: 'Lysis bytecode',
  bytecodeSubtitle: '.lysis',
  exec: 'lysis::execute',
  execSubtitle: 'interpreter',
  sink: 'InterningSink',
  sinkSubtitle: 'hash-cons',
  ssa: 'Vec<Instruction<F>>',
  ssaSubtitle: 'IR-SSA',
  backend: 'R1CS / Plonkish',
};

const esLabels: Labels = {
  proveir: 'ProveIR',
  walker: 'lysis_lift::walker',
  walkerSubtitle: 'BTA · extract · diff · symbolic',
  bytecode: 'bytecode Lysis',
  bytecodeSubtitle: '.lysis',
  exec: 'lysis::execute',
  execSubtitle: 'intérprete',
  sink: 'InterningSink',
  sinkSubtitle: 'hash-cons',
  ssa: 'Vec<Instruction<F>>',
  ssaSubtitle: 'IR-SSA',
  backend: 'R1CS / Plonkish',
};

export const lysisArchitectureSvgEn = build(enLabels).svg;
export const lysisArchitectureSvgEs = build(esLabels).svg;
