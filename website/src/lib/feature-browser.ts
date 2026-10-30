export function setupFeatureBrowser() {
  const panels = [...document.querySelectorAll<HTMLElement>('[data-feature]')];
  const links = [...document.querySelectorAll<HTMLAnchorElement>('[data-feature-link]')];
  const search = document.querySelector<HTMLInputElement>('#feature-search');
  const empty = document.querySelector<HTMLElement>('#feature-empty');
  const count = document.querySelector<HTMLElement>('#feature-count');
  if (!panels.length || !search) return;

  function select(id: string) {
    const chosen = panels.find(panel => panel.dataset.feature === id) ?? panels[0];
    panels.forEach(panel => { panel.hidden = panel !== chosen; });
    links.forEach(link => {
      if (link.dataset.featureLink === chosen.dataset.feature) link.setAttribute('aria-current', 'true');
      else link.removeAttribute('aria-current');
    });
  }
  function fromHash() {
    let id = location.hash.slice(1);
    try { id = decodeURIComponent(id); } catch { /* Fall back for malformed anchors. */ }
    select(id);
  }
  links.forEach(link => link.addEventListener('click', event => {
    if (event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    history.pushState(null, '', link.hash);
    select(link.dataset.featureLink!);
  }));
  search.addEventListener('input', () => {
    const terms = search.value.toLowerCase().trim().split(/\s+/);
    links.forEach(link => { link.hidden = !terms.every(term => link.dataset.search?.includes(term)); });
    const matches = links.filter(link => !link.hidden).length;
    if (empty) empty.hidden = matches !== 0;
    if (count) count.textContent = `${matches} of ${links.length} features`;
  });
  search.addEventListener('keydown', event => {
    if (event.key === 'Escape') { search.value = ''; search.dispatchEvent(new Event('input')); }
    if (event.key === 'Enter') links.find(link => !link.hidden)?.click();
  });
  window.addEventListener('popstate', fromHash);
  window.addEventListener('hashchange', fromHash);
  fromHash();
}
