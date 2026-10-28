# Gitview website

This folder contains the static Astro site for
`https://gitview.barathrajdevarajan.dev`. The site uses Astro 7, Tailwind CSS
v4, and the root repository's current product documentation as its source of
truth.

## Develop and build

Use Bun 1.3.5 and Node.js 24 from this directory.

1. Install the locked dependencies with `bun install --frozen-lockfile`.
2. Start the local site with `bun run dev`.
3. Build static output with `bun run build`.
4. Preview the build with `bun run preview`.

The deployment workflow builds this folder when website files change on
`main`, and it can also be started manually from GitHub Actions. GitHub Pages
must use **GitHub Actions** as its publishing source.

## Deploy to the default GitHub Pages URL

The initial deployment uses the repository URL
`https://barathrajd.github.io/gitview/` so you can confirm the site before
connecting the custom domain.

1. Open the repository's **Settings → Pages**.
2. Set the publishing source to **GitHub Actions**.
3. Push a website change to `main` or start **Deploy website** from Actions.
4. Open the published URL from the successful Pages deployment.

This deployment uses `/gitview` as the site base because the default project
URL includes the repository name.

## Connect the custom domain after the first deployment

After confirming the site at the default URL, add the domain in Pages
settings, then configure DNS. GitHub recommends this order to reduce the risk
of another account claiming an unconfigured subdomain. See
[GitHub's custom-domain instructions](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site).

1. Open **Settings → Pages** and enter
   `gitview.barathrajdevarajan.dev` under **Custom domain**. Save it.
2. In the domain provider's DNS form, add a CNAME with name `gitview` and
   target `barathrajd.github.io`. Keep the provider's default TTL unless you
   have a reason to change it.
3. Tell the site maintainer after Pages accepts the custom domain. The site
   base must switch from `/gitview` to `/` before the next deployment.
4. After DNS propagates, return to **Settings → Pages** and enable **Enforce
   HTTPS** when GitHub makes it available.

DNS propagation can take up to 24 hours. GitHub Actions publishing reads the
custom domain from repository settings; a `CNAME` file in its deployment
artifact is ignored and isn't needed.

## Keep product information current

Before changing feature or platform claims, check the root `README.md`,
`RELEASE.md`, and `Cargo.toml`. The macOS prebuilt installer supports Apple
Silicon. Linux release packaging and Windows support remain deferred. Don't
add performance comparisons unless you have reproducible benchmark data.
