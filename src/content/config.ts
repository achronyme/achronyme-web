import { defineCollection, z } from 'astro:content';
import { glob } from 'astro/loaders';

const docsSchema = z.object({
  title: z.string(),
  description: z.string().optional(),
});

export const collections = {
  'docs-en': defineCollection({
    loader: glob({ pattern: '**/*.mdx', base: './src/content/docs-en' }),
    schema: docsSchema,
  }),
  'docs-es': defineCollection({
    loader: glob({ pattern: '**/*.mdx', base: './src/content/docs-es' }),
    schema: docsSchema,
  }),
};
