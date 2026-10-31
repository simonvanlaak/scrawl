import { parse } from "parse5"

const publicSite = "https://app.tryapparent.com/"

function parsedURL(value, base) {
  try {
    return new URL(value, base)
  } catch {
    return null
  }
}

function isUpstream(url) {
  if (!url || !["http:", "https:"].includes(url.protocol)) return false
  const hostname = url.hostname.toLowerCase().replace(/\.$/, "")
  if (hostname === "apparent.kitlangton.com") return true
  if (!["github.com", "www.github.com", "raw.githubusercontent.com", "codeload.github.com"].includes(hostname)) return false
  let parts
  try {
    parts = url.pathname.split("/").map((part) => decodeURIComponent(part).toLowerCase())
  } catch {
    return false
  }
  return parts[1] === "kitlangton" && ["tap", "homebrew-tap"].includes(parts[2]?.replace(/\.git$/, ""))
}

// Parse HTML attributes (including character references) before interpreting URLs.
// Only destinations count: prose, comments, query strings and lookalike hosts do not.
export function hasUpstreamLinks(html) {
  const document = parse(html)
  const elements = []
  let visibleText = ""
  function visit(node) {
    if (["script", "style"].includes(node.tagName)) return
    if (node.attrs) elements.push(node)
    if (node.nodeName === "#text") visibleText += node.value
    for (const child of node.childNodes ?? []) visit(child)
    // Template links can become active later, so inspect their inert contents too.
    if (node.content) visit(node.content)
    if (node.tagName) visibleText += " "
  }
  visit(document)
  const baseElement = elements.find((node) => node.tagName === "base" && node.attrs.some((attr) => attr.name === "href"))
  const base = parsedURL(baseElement?.attrs.find((attr) => attr.name === "href").value ?? publicSite, publicSite) ?? new URL(publicSite)
  for (const node of elements) {
    for (const attr of node.attrs) {
      const metaURL = node.tagName === "meta" && attr.name === "content" && node.attrs.some((attribute) =>
        ["property", "name"].includes(attribute.name) && ["og:url", "og:image", "twitter:url", "twitter:image"].includes(attribute.value.toLowerCase()))
      if (["href", "src", "action", "formaction", "poster"].includes(attr.name) || metaURL) {
        if (isUpstream(parsedURL(attr.value, base))) return true
      }
    }
  }
  // The old guard also prevented upstream Homebrew install instructions. These
  // are formula tokens, not URLs; keep that protection with token boundaries.
  return /(?:^|[\s'"`])kitlangton\/tap(?:\/[a-z0-9@+_.-]+)?(?=$|[\s'"`])/i.test(visibleText)
}
