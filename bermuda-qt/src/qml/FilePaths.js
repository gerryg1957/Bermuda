.pragma library

// FileDialog and StandardPaths return URLs, not native filesystem paths.
// Decode exactly once, preserve network hosts, and remove the URL-only slash
// before Windows drive letters. POSIX paths must keep their leading slash.
function localPathFromUrl(url, platform) {
    const text = url.toString()
    if (text.length === 0)
        return ""

    const parsed = new URL(text)
    if (parsed.protocol !== "file:")
        return ""

    let path = decodeURIComponent(parsed.pathname)
    const host = parsed.hostname
    if (host.length > 0 && host.toLowerCase() !== "localhost")
        return "//" + host + path

    if (platform === "windows" && /^\/[A-Za-z]:\//.test(path))
        path = path.substring(1)

    return path
}

function directoryPathFromUrl(url, platform) {
    const path = localPathFromUrl(url, platform)
    // C:/ is a drive root. C: alone is a drive-relative path on Windows.
    if (path === "/" || (platform === "windows" && /^[A-Za-z]:\/$/.test(path)))
        return path
    return path.replace(/\/+$/, "")
}
