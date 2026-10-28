# Gitview website

This folder contains the static Astro site for
`https://gitview.barathrajdevarajan.dev`. The site uses Astro 7, Tailwind CSS
v4, and the root repository's current product documentation as its source of
truth.

## Develop and build

Use Node.js 24 and npm from this directory.

1. Install the locked dependencies with `npm ci`.
2. Start the local site with `npm run dev`.
3. Build static output with `npm run build`.
4. Preview the build with `npm run preview`.

The deployment workflow builds this folder when website files change on
`main`, and it can also be started manually from GitHub Actions. GitHub Pages
must use **GitHub Actions** as its publishing source.

## Connect the custom domain

GitHub recommends saving the domain in repository settings before adding its
DNS record. This order reduces the risk of another account claiming an
unconfigured subdomain. See [GitHub's custom-domain instructions](https://docs.github.com/en/pages/configuring-a-custom-domain-for-your-github-pages-site/managing-a-custom-domain-for-your-github-pages-site).

1. Open the repository's **Settings → Pages**.
2. Set the publishing source to **GitHub Actions**.
3. Enter `gitview.barathrajdevarajan.dev` under **Custom domain** and save it.
4. In the domain provider's DNS form, add a CNAME with name `gitview` and
   target `barathrajd.github.io`. Keep the provider's default TTL unless you
   have a reason to change it.
5. After DNS propagates, return to **Settings → Pages** and enable **Enforce
   HTTPS** when GitHub makes it available.

DNS propagation can take up to 24 hours. GitHub Actions publishing reads the
custom domain from repository settings; a `CNAME` file in its deployment
artifact is ignored and isn't needed.

## Keep product information current

Before changing feature or platform claims, check the root `README.md`,
`RELEASE.md`, and `Cargo.toml`. The macOS prebuilt installer supports Apple
Silicon. Linux release packaging and Windows support remain deferred. Don't
add performance comparisons unless you have reproducible benchmark data.
