# Achronyme Web

[![CI](https://github.com/achronyme/achronyme-web/actions/workflows/ci.yml/badge.svg)](https://github.com/achronyme/achronyme-web/actions/workflows/ci.yml)
[![Deploy](https://github.com/achronyme/achronyme-web/actions/workflows/deploy.yml/badge.svg)](https://github.com/achronyme/achronyme-web/actions/workflows/deploy.yml)

Marketing website for [Achronyme](https://github.com/achronyme/achronyme), a programming language for zero-knowledge circuits.

Live at [achrony.me](https://achrony.me).

---

## Stack

- **[Astro 5](https://astro.build)** — Static site generator
- **[Tailwind CSS v4](https://tailwindcss.com)** — Utility-first styling
- **[GSAP](https://gsap.com)** — Scroll and entrance animations
- **Cloudflare Pages** — Hosting and deployment

---

## Development

```bash
npm install
npm run dev        # localhost:4321
npm run build      # production build → dist/
npm run preview    # preview production build
```

---

## Project Structure

```
achronyme-web/
├── src/
│   ├── components/         Astro components
│   │   ├── Navbar.astro
│   │   ├── Hero.astro
│   │   ├── Stats.astro
│   │   ├── Features.astro
│   │   ├── Comparison.astro
│   │   ├── CodeExamples.astro
│   │   └── Footer.astro
│   ├── layouts/
│   │   └── Layout.astro    HTML skeleton, global CSS
│   ├── pages/
│   │   ├── index.astro     English landing page
│   │   └── es/index.astro  Spanish landing page
│   ├── i18n/
│   │   ├── en.json         English translations
│   │   ├── es.json         Spanish translations
│   │   └── index.ts        i18n helper (t, getLocaleFromUrl)
│   └── styles/
│       └── global.css      Tailwind theme, custom utilities
├── public/                 Static assets (favicons, SVGs)
└── .github/workflows/     CI + Cloudflare Pages deploy
```

---

## Internationalization

The site supports English and Spanish. Translations live in `src/i18n/{en,es}.json`. Pages are duplicated under `src/pages/` and `src/pages/es/`.

To add a new language:

1. Create `src/i18n/{lang}.json` with all keys from `en.json`
2. Add the locale to `src/i18n/index.ts`
3. Create `src/pages/{lang}/index.astro`

---

## Design

- **Dark theme** — void `#0A0A0F`, surface `#13131A`, subtle `#1E1E2A`
- **Accent** — proof purple `#A855F7`
- **Font** — JetBrains Mono (monospace-first)
- **Effects** — dot-grid background, glow-card hover, scroll reveal, gradient connectors

---

## License

GPL-3.0
