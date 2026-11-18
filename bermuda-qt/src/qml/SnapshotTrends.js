// Pure data preparation shared by the QML graph and its regression tests.
function settings(snapshots, selected) {
    const result = []
    snapshots.filter(function(s) { return selected.indexOf(s.id) >= 0 }).forEach(function(s) {
        (s.trend_series || []).forEach(function(g) {
            if (!result.some(function(item) { return item.key === g.key }))
                result.push({key: g.key, label: "Set " + (result.length + 1) + " · " + g.visits + " visits · " + g.rules + " · komi " + g.komi + " · " + g.board_size + "×" + g.board_size})
        })
    })
    return result
}
function points(snapshots, selected, key, bucket) {
    const result = []
    snapshots.filter(function(s) { return selected.indexOf(s.id) >= 0 }).forEach(function(s) {
        const g = (s.trend_series || []).find(function(item) { return item.key === key })
        if (!g || !g.decisions || !g.from || !g.to || g.undated) return
        const counts = g.buckets || []
        if (counts.length !== 4 || counts.reduce(function(a, b) { return a + b }, 0) !== g.decisions) return
        result.push({id:s.id, groupId:s.group_id, name:s.group_name, label:s.label,
            from:g.from, to:g.to, time:Date.parse(g.to + "T00:00:00Z"),
            games:g.games, complete:g.complete, moves:g.decisions, ids:g.game_ids,
            percent:100 * counts[bucket] / g.decisions,
            pending:(s.report.pending || []).length, excluded:s.report.excluded || 0})
    })
    result.sort(function(a, b) { return a.time - b.time || a.id - b.id })
    return result
}
function overlaps(points) {
    const seen = {}
    for (let i = 0; i < points.length; ++i) {
        for (let j = 0; j < points[i].ids.length; ++j) {
            const id = points[i].ids[j]
            if (seen[id]) return true
            seen[id] = true
        }
    }
    return false
}
function choose(snapshots, selected, id, checked) {
    const report = snapshots.find(function(s) { return s.id === id })
    return selected.filter(function(old) {
        const s = snapshots.find(function(s) { return s.id === old })
        return s && old !== id && (!checked || s.group_id !== report.group_id)
    }).concat(checked ? [id] : [])
}
