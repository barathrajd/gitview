# Gitview product website

This plan covers a static product and installation site for the existing
Gitview CLI.

## Objective

Give developers evaluating Gitview a clear view of its current terminal Git
review workflows and a direct path to install it. Publish a responsive,
single-page Astro site at `https://gitview.barathrajdevarajan.dev`, hosted on
GitHub Pages without changing the Rust CLI or its release process.

## Success criteria

The implementation is complete when these outcomes can be checked directly.

- The site builds as a static Astro site and deploys from GitHub Actions to
  GitHub Pages using the custom domain configuration.
- The page has responsive navigation, a product hero, a CSS-rendered Gitview
  terminal preview, current feature summaries, installation instructions, and
  a footer with source and release links.
- The primary install command is `cargo install gitview --locked`; the guide
  also explains the macOS Apple Silicon release archive and its supported
  install path.
- Every feature and platform statement matches the repository README or a
  released capability. The page makes no unsupported speed comparison or
  benchmark claim.
- The terminal preview accurately depicts supported history, working-tree,
  diff, hunk staging, local branch, and profile workflows; it does not imply
  remote operations, commit creation, or interactive rebase support.
- The install tabs and copy command work with keyboard and pointer input, and
  copy feedback is announced accessibly.
- The page includes page title, description, canonical URL, Open Graph metadata,
  and a favicon; links point to the actual project repository, crate, and
  releases.
- The web app and its dependencies live under `website/`; Cargo commands and
  existing release automation continue to work from the repository root.

## In scope / Out of scope

The site presents the current product and its supported installation paths.

In scope:

- A static, single-page product website in an isolated `website/` Astro
  project, styled with Tailwind CSS v4 utilities, Inter and JetBrains Mono,
  and a dark terminal palette.
- Reusable Astro components for base layout, navigation, hero, terminal
  preview, feature grid, install guide, and footer.
- A CSS-only faux terminal based on the existing Gitview UI, with no external
  video or heavy client-side framework.
- Install choices for Cargo and the macOS Apple Silicon release archive,
  with copyable commands and links to GitHub source and releases.
- A GitHub Pages Actions workflow, custom-domain `CNAME`, and deployment
  documentation or checklist for the required repository and DNS settings.
- Responsive layout, semantic HTML, reduced-motion support, accessible
  keyboard behavior, and SEO/social metadata.

Out of scope:

- Changes to Gitview's Rust application, CLI, feature set, package metadata,
  installer behavior, or existing release workflow.
- Brew or Go installation instructions, a `gitview.dev` domain, or the
  unsupported `curl https://gitview.dev | sh` installer command.
- Performance benchmark numbers or claims that Gitview is faster than
  another tool.
- Claims or UI implying push/pull, remote management, commit creation,
  interactive rebase, conflict resolution, Linux release packaging, or
  Windows support.
- Additional pages, user accounts, server-side rendering, analytics, or a
  blog/CMS.

## Approach

Create a self-contained static Astro project under `website/`, using the
current stable Astro 7 line and Tailwind CSS v4 through its documented Vite
plugin. Astro's official GitHub Pages guide supports prerendered static output,
an Actions deployment, and custom domains; its custom-domain setup uses a
`public/CNAME` file and DNS configuration. Tailwind's Astro guide documents
the Vite plugin integration and global CSS import. [Astro GitHub Pages guide](https://docs.astro.build/en/guides/deploy/github/)
and [Tailwind Astro guide](https://tailwindcss.com/docs/installation/framework-guides/astro).
Use the [Astro 7 release line](https://github.com/withastro/astro/releases)
and the official integration guidance as the version references at build time.

Keep rendering static and add only small client scripts for the install tabs
and clipboard feedback. Use `cargo install gitview --locked` as the primary
CTA, with the existing macOS Apple Silicon archive path as the alternate
choice. Build page content from `README.md`, `Cargo.toml`, and `RELEASE.md`;
omit the requested benchmark section because the repository has no verified
benchmark data. Use a CSS terminal mockup to show current workflows, not an
aspirational feature set. Use the GitHub Pages workflow with a `website/` build
path so root-level Rust development and release automation remain independent.

## Milestones

Each milestone leaves the site in a buildable state and adds one verifiable
part of the final experience.

1. [x] Scaffold and build the isolated Astro/Tailwind site with shared layout,
   global theme, SEO metadata, and custom-domain configuration. Verify the
   static output builds from `website/`.
2. Publish the product story in a responsive hero and terminal preview that
   show supported history, worktree, diff, and staging workflows. Review all
   displayed claims against the current README and release notes.
3. Add the feature grid and interactive installation guide with Cargo as the
   primary path, a macOS Apple Silicon release option, accessible tabs, and
   clipboard feedback. Verify links and keyboard interaction.
4. Add the footer, GitHub Pages workflow, and custom-domain file. Verify the
   Actions build and deployment configuration; complete the DNS and Pages
   settings to publish at the target subdomain.

## Risks & open questions

The remaining release dependency is configuring the custom domain outside the
site source.

- **Custom-domain activation — owner: user.** GitHub Pages needs DNS records
  for the subdomain and Pages enabled for this repository. The workflow and
  `CNAME` can be committed independently; publish after the DNS and repository
  settings are in place.
- **Release support can change — owner: site maintainer.** Keep install
  instructions and platform claims grounded in `README.md` and `RELEASE.md`,
  and update them when release artifacts change.
- **Google Fonts availability — owner: site maintainer.** If remote font
  loading is blocked or undesirable, fall back to system sans-serif and
  monospace stacks without affecting layout or readability.

## Boundaries

The website work stays separate from the product implementation and release
process.

- Preserve the Rust package, CLI behavior, root Cargo workflows, and current
  release automation.
- Do not add or imply Git operations that Gitview does not currently support.
- Do not publish unverified performance metrics, platform availability, or
  install commands.
- Keep the site statically renderable and its browser-side behavior limited to
  lightweight install-tab and copy interactions.
