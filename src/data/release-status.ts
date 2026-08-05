export const releaseStatus = {
  core: {
    stable: '0.0.1',
    candidate: '0.1.0',
    candidateRevision: 'fd07b38e16256e2ed6a8f2b438d340a681c9b0ac',
    stableUrl: 'https://github.com/achronyme/achronyme/releases/tag/v0.0.1',
    releasesUrl: 'https://github.com/achronyme/achronyme/releases',
  },
  editor: {
    stable: '0.2.0',
    candidate: '0.3.0',
    stableUrl: 'https://github.com/achronyme/achronyme-editor/releases/tag/v0.2.0',
    repositoryUrl: 'https://github.com/achronyme/achronyme-editor',
  },
  web: {
    candidate: '0.1.0',
    playgroundUrl: 'https://achrony.me/playground',
    repositoryUrl: 'https://github.com/achronyme/achronyme-web',
  },
} as const;

export const releaseDocs = {
  en: '/docs/releases/0.1.0',
  es: '/es/docs/releases/0.1.0',
} as const;
