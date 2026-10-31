import { parse } from "parse5"

// Extract document body text for the stale-build check, not sanitized HTML.
// The result may contain literal angle brackets and must never be rendered as HTML.
export function packagePageText(html) {
  const document = parse(html)
  const root = document.childNodes.find((node) => node.tagName === "html")
  const body = root?.childNodes.find((node) => node.tagName === "body")
  const text = []
  function visit(node) {
    if (["script", "style", "template"].includes(node.tagName)) return
    if (node.nodeName === "#text") text.push(node.value)
    for (const child of node.childNodes ?? []) visit(child)
  }
  if (body) visit(body)
  return text.join("")
}
