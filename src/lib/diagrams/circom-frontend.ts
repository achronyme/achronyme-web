/**
 * Circom frontend pipeline diagram (TB).
 *
 * Migrated from the ASCII art block in
 * docs-{en,es}/architecture/circom-frontend.mdx.
 */

import { flowchart } from '@achronyme/diagrams';

interface Labels {
  source: string;
  tokens: string;
  tokensSubtitle: string;
  ast: string;
  astSubtitle: string;
  validated: string;
  validatedSubtitle: string;
  lowered: string;
  loweredSubtitle: string;
  ir: string;
  irSubtitle: string;
  constraints: string;
  constraintsSubtitle: string;
}

function build(L: Labels) {
  return flowchart()
    .node('source', { label: L.source, shape: 'data' })
    .node('tokens', { label: L.tokens, subtitle: L.tokensSubtitle })
    .node('ast', { label: L.ast, subtitle: L.astSubtitle })
    .node('validated', { label: L.validated, subtitle: L.validatedSubtitle })
    .node('lowered', { label: L.lowered, subtitle: L.loweredSubtitle })
    .node('ir', { label: L.ir, subtitle: L.irSubtitle })
    .node('constraints', { label: L.constraints, subtitle: L.constraintsSubtitle, shape: 'terminator' })
    .edge('source', 'tokens')
    .edge('tokens', 'ast')
    .edge('ast', 'validated')
    .edge('validated', 'lowered')
    .edge('lowered', 'ir')
    .edge('ir', 'constraints')
    .render();
}

const enLabels: Labels = {
  source: '.circom source',
  tokens: 'Tokens',
  tokensSubtitle: 'circom::lexer',
  ast: 'Circom AST',
  astSubtitle: 'circom::parser',
  validated: 'Validated AST',
  validatedSubtitle: 'circom::analysis · diagnostics',
  lowered: 'ProveIR + Artik bytecode',
  loweredSubtitle: 'circom::lowering',
  ir: 'SSA IR',
  irSubtitle: 'ProveIR instantiate',
  constraints: 'Constraints',
  constraintsSubtitle: 'R1CS backend',
};

const esLabels: Labels = {
  source: 'fuente .circom',
  tokens: 'Tokens',
  tokensSubtitle: 'circom::lexer',
  ast: 'AST Circom',
  astSubtitle: 'circom::parser',
  validated: 'AST validado',
  validatedSubtitle: 'circom::analysis · diagnostics',
  lowered: 'ProveIR + bytecode Artik',
  loweredSubtitle: 'circom::lowering',
  ir: 'SSA IR',
  irSubtitle: 'ProveIR instanciar',
  constraints: 'Restricciones',
  constraintsSubtitle: 'backend R1CS',
};

export const circomFrontendSvgEn = build(enLabels).svg;
export const circomFrontendSvgEs = build(esLabels).svg;
