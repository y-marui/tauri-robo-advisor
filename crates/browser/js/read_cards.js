(mode, wanted, depth) => {
  // Reads repeated blocks ("cards") under one heading. It only reads: querySelectorAll,
  // getComputedStyle, getClientRects, and innerText. No class name, attribute, or link target is
  // used, and nothing is clicked or typed.
  const MAX_HEADINGS = 50, MAX_HEADING_CHARS = 60, MAX_ITEMS = 500, MAX_LINES = 20, MAX_LINE_CHARS = 1000;
  const MAX_CLIMB = 3;
  const squash = (t) => String(t ?? '').replace(/[ \t\r\n\f\v - 　]+/g, ' ').trim();
  const lines = (t) => String(t ?? '').split('\n').map((l) => l.replace(/[ \t\r\f\v - 　]+/g, ' ').trim()).filter((l) => l !== '');
  const shown = (e) => {
    if (e.getClientRects().length === 0) return false;
    for (let n = e; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.visibility === 'hidden' || s.display === 'none' || Number(s.opacity) === 0) return false;
    }
    return true;
  };
  const textOf = (e) => String(e.innerText ?? e.textContent ?? '');
  const isHeading = (e) => /^H[1-6]$/.test(e.tagName);
  const visible = [...document.querySelectorAll('h1, h2, h3, h4, h5, h6')]
    .filter(shown)
    .map((element) => ({ element, text: squash(textOf(element)) }))
    .filter((h) => h.text !== '');
  if (mode === 'list') {
    return {
      headings: visible.slice(0, MAX_HEADINGS).map((h) => h.text.slice(0, MAX_HEADING_CHARS)),
      cut: visible.length > MAX_HEADINGS,
    };
  }
  const start = squash(wanted);
  const found = start === '' ? [] : visible.filter((h) => h.text.startsWith(start));
  if (found.length === 0) return { error: 'none' };
  if (found.length > 1) return { error: 'many' };
  // A heading alone in a wrapper stands for the wrapper (a few levels at most).
  let anchor = found[0].element;
  for (let i = 0; i < MAX_CLIMB; i++) {
    const parent = anchor.parentElement;
    if (!parent || parent.children.length !== 1) break;
    anchor = parent;
  }
  // The region is the first block with text after it, unless a heading comes first.
  let region = null;
  for (let e = anchor.nextElementSibling; e; e = e.nextElementSibling) {
    if (isHeading(e)) break;
    if (shown(e) && squash(textOf(e)) !== '') { region = e; break; }
  }
  if (!region) return { items: [], cut: false };
  const blocks = (e) => [...e.children].filter((c) => shown(c) && squash(textOf(c)) !== '');
  const steps = Math.min(Math.max(Number(depth) || 1, 1), 3);
  let level = [region];
  for (let i = 0; i < steps; i++) level = level.flatMap(blocks);
  if (level.length === 1 && blocks(level[0]).length > 1) return { error: 'shallow' };
  let cut = level.length > MAX_ITEMS;
  const items = [];
  for (const block of level.slice(0, MAX_ITEMS)) {
    const parts = lines(textOf(block));
    if (parts.length > MAX_LINES) cut = true;
    items.push(parts.slice(0, MAX_LINES).map((l) => { if (l.length > MAX_LINE_CHARS) cut = true; return l.slice(0, MAX_LINE_CHARS); }));
  }
  return { items, cut };
}
