// The appcast has no Apparent release yet. A new signed release enables the Mac link.
import appcast from "../public/appcast.xml?raw"
import { latestRelease } from "./appcast"

export const RELEASE = latestRelease(appcast)
export const MAC_DMG: string | null = RELEASE.version ? "/releases/Apparent-latest-arm64.dmg" : null
export const BREW = "brew install --cask tryapparent/tap/apparent"
export const SKILL = "bunx skills add tryapparent/apparent"

// The zip .github/workflows/release-windows.yml builds for the same version,
// copied next to the Mac files. A preview: unsigned, and it doesn't update
// itself yet. Set it to null and the site says "coming soon" again.
export const WINDOWS_DOWNLOAD: string | null = null

// Enable each platform only after its artifact has been published.
export const LINUX_DOWNLOAD: string | null = null
