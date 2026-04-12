export interface SidebarItem {
  label: string;
  slug: string;
  translations?: Record<string, string>;
}

export interface SidebarGroup {
  label: string;
  translations?: Record<string, string>;
  items: SidebarItem[];
}

export type SidebarConfig = SidebarGroup[];

export const sidebarConfig: SidebarConfig = [
  {
    label: 'Getting Started',
    translations: { es: 'Primeros Pasos' },
    items: [
      { label: 'Introduction', slug: 'getting-started/introduction', translations: { es: 'Introduccion' } },
      { label: 'Installation', slug: 'getting-started/installation', translations: { es: 'Instalacion' } },
      { label: 'Hello World', slug: 'getting-started/hello-world', translations: { es: 'Hola Mundo' } },
      { label: 'Editor Setup', slug: 'getting-started/editor-setup', translations: { es: 'Configuracion del Editor' } },
    ],
  },
  {
    label: 'Language Reference',
    translations: { es: 'Referencia del Lenguaje' },
    items: [
      { label: 'Types & Values', slug: 'language/types-and-values', translations: { es: 'Tipos y Valores' } },
      { label: 'Control Flow', slug: 'language/control-flow', translations: { es: 'Flujo de Control' } },
      { label: 'Functions & Closures', slug: 'language/functions-and-closures', translations: { es: 'Funciones y Closures' } },
      { label: 'Arrays & Collections', slug: 'language/arrays-and-collections', translations: { es: 'Arrays y Colecciones' } },
      { label: 'Native Functions', slug: 'language/native-functions', translations: { es: 'Funciones Nativas' } },
      { label: 'Methods & Static Namespaces', slug: 'language/methods', translations: { es: 'Metodos y Namespaces Estaticos' } },
      { label: 'Error Handling', slug: 'language/error-handling', translations: { es: 'Manejo de Errores' } },
      { label: 'Diagnostics & Warnings', slug: 'language/diagnostics', translations: { es: 'Diagnosticos y Advertencias' } },
      { label: 'Modules', slug: 'language/modules', translations: { es: 'Modulos' } },
    ],
  },
  {
    label: 'Circuit Programming',
    translations: { es: 'Programacion de Circuitos' },
    items: [
      { label: 'Overview', slug: 'circuits/overview', translations: { es: 'Descripcion General' } },
      { label: 'Declarations', slug: 'circuits/declarations', translations: { es: 'Declaraciones' } },
      { label: 'Type Annotations', slug: 'circuits/type-annotations', translations: { es: 'Anotaciones de Tipo' } },
      { label: 'Builtins', slug: 'circuits/builtins', translations: { es: 'Funciones Integradas' } },
      { label: 'Operators & Costs', slug: 'circuits/operators-and-costs', translations: { es: 'Operadores y Costos' } },
      { label: 'Functions in Circuits', slug: 'circuits/functions', translations: { es: 'Funciones en Circuitos' } },
      { label: 'Control Flow in Circuits', slug: 'circuits/control-flow', translations: { es: 'Flujo de Control en Circuitos' } },
    ],
  },
  {
    label: 'Circom Interop',
    translations: { es: 'Interoperabilidad con Circom' },
    items: [
      { label: 'Overview', slug: 'circom/overview', translations: { es: 'Descripcion General' } },
      { label: 'Importing Templates', slug: 'circom/importing', translations: { es: 'Importando Templates' } },
      { label: 'Circuit Mode', slug: 'circom/circuit-mode', translations: { es: 'Modo Circuito' } },
      { label: 'VM Mode', slug: 'circom/vm-mode', translations: { es: 'Modo VM' } },
      { label: 'Diagnostics', slug: 'circom/diagnostics', translations: { es: 'Diagnosticos' } },
      { label: 'Limitations & Roadmap', slug: 'circom/limitations', translations: { es: 'Limitaciones y Roadmap' } },
    ],
  },
  {
    label: 'Zero-Knowledge Concepts',
    translations: { es: 'Conceptos de Conocimiento Cero' },
    items: [
      { label: 'Field Elements', slug: 'zk-concepts/field-elements', translations: { es: 'Elementos de Campo' } },
      { label: 'R1CS', slug: 'zk-concepts/r1cs' },
      { label: 'Plonkish', slug: 'zk-concepts/plonkish' },
      { label: 'Proof Generation', slug: 'zk-concepts/proof-generation', translations: { es: 'Generacion de Pruebas' } },
    ],
  },
  {
    label: 'CLI Reference',
    translations: { es: 'Referencia del CLI' },
    items: [
      { label: 'Commands', slug: 'cli/commands', translations: { es: 'Comandos' } },
      { label: 'Project Configuration', slug: 'cli/project-config', translations: { es: 'Configuracion del Proyecto' } },
      { label: 'Circuit Options', slug: 'cli/circuit-options', translations: { es: 'Opciones de Circuito' } },
    ],
  },
  {
    label: 'Architecture',
    translations: { es: 'Arquitectura' },
    items: [
      { label: 'Pipeline Overview', slug: 'architecture/pipeline', translations: { es: 'Vision General del Pipeline' } },
      { label: 'Crate Map', slug: 'architecture/crate-map', translations: { es: 'Mapa de Crates' } },
      { label: 'IR & Optimization', slug: 'architecture/ir-and-optimization', translations: { es: 'IR y Optimizacion' } },
      { label: 'Backends', slug: 'architecture/backends' },
      { label: 'Witness Generation', slug: 'architecture/witness-generation', translations: { es: 'Generacion de Testigos' } },
      { label: 'Extension Guide', slug: 'architecture/extension-guide', translations: { es: 'Guia de Extension' } },
      { label: 'VM & Bytecode', slug: 'architecture/vm-and-bytecode', translations: { es: 'VM y Bytecode' } },
      { label: 'Memory & GC', slug: 'architecture/memory-and-gc', translations: { es: 'Memoria y GC' } },
    ],
  },
  {
    label: 'Playground',
    items: [
      { label: 'Overview', slug: 'playground/overview', translations: { es: 'Descripcion General' } },
      { label: 'IDE Features', slug: 'playground/ide-features', translations: { es: 'Features del IDE' } },
      { label: 'Projects & Sessions', slug: 'playground/projects', translations: { es: 'Proyectos y Sesiones' } },
    ],
  },
  {
    label: 'Tutorials',
    translations: { es: 'Tutoriales' },
    items: [
      { label: 'Merkle Membership Proof', slug: 'tutorials/merkle-proof', translations: { es: 'Prueba de Membresia Merkle' } },
      { label: 'Inline Proofs', slug: 'tutorials/inline-proofs', translations: { es: 'Pruebas en Linea' } },
      { label: 'Poseidon Hashing', slug: 'tutorials/poseidon-hashing', translations: { es: 'Hashing Poseidon' } },
      { label: 'BigInt Arithmetic', slug: 'tutorials/bigint-arithmetic', translations: { es: 'Aritmetica BigInt' } },
      { label: 'Secret Voting', slug: 'tutorials/secret-voting', translations: { es: 'Votacion Secreta' } },
    ],
  },
  {
    label: 'Releases',
    translations: { es: 'Releases' },
    items: [
      { label: 'Changelog', slug: 'releases/changelog' },
    ],
  },
];

export function getLabel(item: { label: string; translations?: Record<string, string> }, locale: string): string {
  if (locale === 'en') return item.label;
  return item.translations?.[locale] ?? item.label;
}

export function flatItems(sidebar: SidebarConfig): SidebarItem[] {
  return sidebar.flatMap(g => g.items);
}

export function getPrevNext(currentSlug: string): { prev: SidebarItem | null; next: SidebarItem | null } {
  const flat = flatItems(sidebarConfig);
  const idx = flat.findIndex(i => i.slug === currentSlug);
  return {
    prev: idx > 0 ? flat[idx - 1] : null,
    next: idx < flat.length - 1 ? flat[idx + 1] : null,
  };
}
