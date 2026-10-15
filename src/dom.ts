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
