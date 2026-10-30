// Shows one release at a time on pages with a version list: articles carry
// data-release="v0-7-0" and links carry data-release-link="v0-7-0". The URL's
// #v0-7-0 anchor chooses one (the first by default), so links can be shared
// and the back button works. Without JavaScript every release stays visible.
export function setupReleaseSwitcher() {
  const articles = Array.from(document.querySelectorAll<HTMLElement>("[data-release]"));
  const links = Array.from(document.querySelectorAll<HTMLAnchorElement>("[data-release-link]"));
  const panel = document.getElementById("release-panel");
  if (!articles.length) return;

  function show(id: string | undefined) {
    const chosen = articles.find((article) => article.dataset.release === id) ?? articles[0];
    articles.forEach((article) => {
      article.hidden = article !== chosen;
    });
    links.forEach((link) => {
      const active = link.dataset.releaseLink === chosen.dataset.release;
      link.classList.toggle("is-active", active);
      link.classList.toggle("text-emerald-300", active);
      link.classList.toggle("border-emerald-300/40", active);
      link.classList.toggle("lg:border-emerald-400", active);
      if (active) {
        link.setAttribute("aria-current", "true");
        // Bring the chip into view in the phone's scrolling row only.
        const row = link.closest("ol");
        if (row && row.scrollWidth > row.clientWidth) {
          row.scrollLeft = link.offsetLeft - row.clientWidth / 2 + link.offsetWidth / 2;
        }
      } else {
        link.removeAttribute("aria-current");
      }
    });
    document.dispatchEvent(new CustomEvent("release-shown", { detail: chosen.dataset.release }));
  }

  // Switch without the browser's anchor jump, which would land on a position
  // that moves once the other releases are hidden.
  links.forEach((link) => {
    link.addEventListener("click", (event) => {
      event.preventDefault();
      const id = link.dataset.releaseLink;
      history.pushState(null, "", `#${id}`);
      show(id);
      if (panel && panel.getBoundingClientRect().top < 0) panel.scrollIntoView({ block: "start" });
    });
  });

  // An anchor can also name a section inside a release, such as #sessions:
  // open the release that holds it and scroll to the section.
  function fromHash() {
    const hash = decodeURIComponent(window.location.hash.slice(1));
    if (!hash) return show(undefined);
    if (articles.some((article) => article.dataset.release === hash)) {
      show(hash);
      panel?.scrollIntoView({ block: "start" });
      return;
    }
    const target = document.getElementById(hash);
    const holder = target?.closest<HTMLElement>("[data-release]");
    show(holder?.dataset.release);
    target?.scrollIntoView({ block: "start" });
  }

  window.addEventListener("popstate", fromHash);
  fromHash();
}
