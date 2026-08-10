export const releaseStatus = {
  core: {
    stable: '0.1.2',
    stableRevision: 'cd7a6e66e133bebd8e2026e321a4c85023c311f7',
    stableUrl: 'https://github.com/achronyme/achronyme/releases/tag/v0.1.2',
    releasesUrl: 'https://github.com/achronyme/achronyme/releases',
  },
  editor: {
    stable: '0.3.1',
    stableUrl: 'https://github.com/achronyme/achronyme-editor/releases/tag/v0.3.1',
    repositoryUrl: 'https://github.com/achronyme/achronyme-editor',
  },
  web: {
    stable: '0.1.2',
    playgroundUrl: 'https://achrony.me/playground',
    repositoryUrl: 'https://github.com/achronyme/achronyme-web',
  },
} as const;

export const releaseDocs = {
  en: '/docs/releases/0.1.2',
  es: '/es/docs/releases/0.1.2',
} as const;
