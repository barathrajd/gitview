const configured = import.meta.env.BASE_URL;
export const base = configured.endsWith("/") ? configured : `${configured}/`;

/** Build a site-internal link that works at the root and under a project path. */
export const href = (path = "") => `${base}${path}`;
