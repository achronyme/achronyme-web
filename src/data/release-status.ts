export const releaseStatus = {
  core: {
    stable: '0.1.1',
    stableRevision: 'b1774e88671a1889146804cb812cb099eb9cc006',
    stableUrl: 'https://github.com/achronyme/achronyme/releases/tag/v0.1.1',
    releasesUrl: 'https://github.com/achronyme/achronyme/releases',
  },
  editor: {
    stable: '0.3.1',
    stableUrl: 'https://github.com/achronyme/achronyme-editor/releases/tag/v0.3.1',
    repositoryUrl: 'https://github.com/achronyme/achronyme-editor',
  },
  web: {
    stable: '0.1.1',
    playgroundUrl: 'https://achrony.me/playground',
    repositoryUrl: 'https://github.com/achronyme/achronyme-web',
  },
} as const;

export const releaseDocs = {
  en: '/docs/releases/0.1.1',
  es: '/es/docs/releases/0.1.1',
} as const;
