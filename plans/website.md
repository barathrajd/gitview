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
  GitHub Pages with the custom domain configured in repository settings.
- The page has responsive navigation, a product hero, a CSS-rendered Gitview
  terminal preview, current feature summaries, installation instructions, and
  a footer with source and release links.
- The primary install command is `cargo install gitview --locked`; the guide
  also explains the macOS Apple Silicon release archive and its supported
  install path.
- Every feature and platform statement matches the repository README or a
  released capability. The page makes no unsupported speed comparison or
  benchmark claim.
- The terminal preview matches the current Worktree, History, and Inspector
  layout and key hints; its repository content is labeled as sample data.
  Feature descriptions cover diffs, staging, and commit creation and amend, as
  shipped in 0.5.0, without implying remote operations or interactive rebase
  support.
- The home page stays high level: one terminal screenshot in the hero, the
  six-card feature summary, and the install guide, with links to the other pages.
- A `/features/` page shows each workflow with a screenshot of the real binary.
  Its "Coming in 0.6.0" section is labeled planned and not yet released, and the
  install instructions stay on 0.5.0 until the release is published.
- A `/changelog/` page lists every release from 0.2.1, with the planned 0.6.0
  clearly marked, summarized from `RELEASE.md` in `website/src/data/releases.ts`. It states that
  Linux packaging moves to the next release and that remotes, merge, and export
  stay out.
- The install tabs and copy command work with keyboard and pointer input, and
  copy feedback is announced accessibly.
- The page includes page title, description, canonical URL, Open Graph metadata,
  and a favicon; links point to the actual project repository, crate, and
  releases.
- The web app and its Bun-managed dependencies live under `website/`; Cargo
  commands and existing release automation continue to work from the root.

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
- A GitHub Pages Actions workflow and deployment checklist for the required
  repository and DNS settings.
- Responsive layout, semantic HTML, reduced-motion support, accessible
  keyboard behavior, and SEO/social metadata.

Out of scope:

- Changes to Gitview's Rust application, CLI, feature set, package metadata,
  installer behavior, or existing release workflow.
- Brew or Go installation instructions, a `gitview.dev` domain, or the
  unsupported `curl https://gitview.dev | sh` installer command.
- Performance benchmark numbers or claims that Gitview is faster than
  another tool.
- Claims or UI implying push/pull, remote management, interactive rebase,
  conflict resolution, Linux release packaging, or Windows support. Commit
  creation and amend are shipped and may be described; 0.6.0 features may be
  described only as planned until release.
- Additional pages, user accounts, server-side rendering, analytics, or a
  blog/CMS.

## Approach

Create a self-contained static Astro project under `website/`, using Bun 1.3.5
with a committed `bun.lock`, the current stable Astro 7 line, and Tailwind CSS
v4 through its documented Vite plugin. The initial GitHub Pages deploy uses
the project path `/gitview` and `barathrajd.github.io`; once the custom domain
is configured, remove that path and build with the custom domain as `site`.
Astro's official GitHub Pages guide supports prerendered static output
and Actions deployment. For this custom Actions workflow, configure the domain
in repository Pages settings; GitHub documents that a deployed `CNAME` file is
ignored for custom workflow publishing. Add DNS only after saving the domain in
Pages settings. Tailwind's Astro guide documents the Vite plugin integration
and global CSS import. [Astro GitHub Pages guide](https://docs.astro.build/en/guides/deploy/github/),
[Tailwind Astro guide](https://tailwindcss.com/docs/installation/framework-guides/astro),
the [Astro Bun guide](https://docs.astro.build/en/recipes/bun/), and [GitHub custom-domain guide](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site).
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
   global theme, SEO metadata, and the site's canonical domain. Verify the
   static output builds from `website/`.
2. [x] Publish the product story in a responsive hero and terminal preview that
   matches the app's Worktree, History, and Inspector panes. Describe diff and
   staging workflows in the feature sections; review all claims against the
   current README and release notes.
3. [x] Add the feature grid and interactive installation guide with Cargo as the
   primary path, a macOS Apple Silicon release option, accessible tabs, and
   clipboard feedback. Verify links and keyboard interaction.
4. [x] Add the footer, Bun-based GitHub Pages workflow, and deployment
   checklist. Deploy at the default project URL first, then switch to the
   custom domain and root base after the user adds the domain in Pages settings.

5. [x] Add the "Coming in 0.6.0" section to a separate `/features/` page, and a
   `/changelog/` page for all releases, so the home page keeps one hero
   screenshot and only the high-level feature cards. Links use a base-path
   helper so they work at the root and under `/gitview`. Add one screenshot per planned
   workflow: repository setup (real command output), path operations, commit and
   amend, history filter/search/comparison, branches and tags, object
   description, and help. Each screen is captured from the real development
   binary in a pseudo-terminal at 140x24 on a throwaway sample repository, and
   converted to an inline SVG with the true colors and a JetBrains Mono grid; no
   screen is drawn by hand. Label the section planned, keep the install guide on
   0.5.0, and add a README section that matches. After 0.6.0 is published,
   recapture the screens from the released binary, retitle the section as
   shipped, and update the install guide and version badge. Add each new
   release to `releases.ts` when it ships.

## Risks & open questions

- **Screenshots can go stale — owner: site maintainer.** The 0.6.0 screens come
  from a development build. Recapture them from the released binary before
  calling them shipped, and rerun the capture if a layout or key hint changes.

The remaining release dependency is configuring the custom domain outside the
site source.

- **Custom-domain activation — owner: user/site maintainer.** Save the custom
  domain in Pages settings before adding the CNAME pointing to
  `barathrajd.github.io`. GitHub Actions custom publishing ignores a `CNAME`
  file, so the domain setting is the source of truth.
- **Two-stage base path — owner: site maintainer.** The first deployment uses
  `/gitview`; after Pages accepts the custom domain, redeploy with the root
  base path so styles and links resolve at the custom-domain root.
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
