// Tiny DOM helpers. Text is always set through `textContent` or text nodes,
// never as HTML, so user-entered names cannot inject markup.

export type Child = Node | string | null | undefined | false;
export type Attrs = Record<string, string | number | boolean | undefined>;

export function h<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Attrs = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  for (const [name, value] of Object.entries(attrs)) {
    if (value === undefined || value === false) {
      continue;
    }
    if (name === "class") {
      element.className = String(value);
    } else {
      element.setAttribute(name, value === true ? "" : String(value));
    }
  }
  for (const child of children) {
    if (child === null || child === undefined || child === false) {
      continue;
    }
    element.append(typeof child === "string" ? document.createTextNode(child) : child);
  }
  return element;
}

/** A small "i" that shows `text` on hover and on keyboard focus (and to screen readers). */
export function infoIcon(text: string): HTMLElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
  path.setAttribute("d", "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20zm1 15h-2v-6h2v6zm0-8h-2V7h2v2z");
  path.setAttribute("fill", "currentColor");
  svg.append(path);
  const icon = h("span", { class: "info-icon", tabindex: 0, role: "img", "aria-label": text, "data-tip": text });
  icon.append(svg);
  return icon;
}
