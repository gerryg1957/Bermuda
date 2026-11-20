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
// Aggregate counts, never average percentages from unequal game groups.
function combined(groups) {
    const valid = groups.filter(function(g) {
        const counts=g.buckets || []
        return g.decisions>0 && !g.undated && isFinite(day(g.from)) && isFinite(day(g.to))
            && counts.length===4 && counts.every(function(n) {return Number.isFinite(n) && n>=0})
            && counts.reduce(function(a,b) {return a+b},0)===g.decisions
    })
    if (!valid.length) return null
    const result={games:0,complete:0,decisions:0,buckets:[0,0,0,0],game_ids:[],visits:[],undated:0,
        from:valid[0].from,to:valid[0].to}
    for (let i=0;i<valid.length;i++) {
        const g=valid[i], ids=g.game_ids || []
        // Current snapshots contain one run per game. Refuse malformed evidence
        // with duplicate games rather than count a game's moves twice.
        if (ids.length!==g.games || ids.some(function(id) {return result.game_ids.indexOf(id)>=0})
            || new Set(ids).size!==ids.length) return null
        result.game_ids=result.game_ids.concat(ids)
        result.games+=g.games;result.complete+=g.complete;result.decisions+=g.decisions
        for (let b=0;b<4;b++) result.buckets[b]+=g.buckets[b]
        if (result.visits.indexOf(g.visits)<0) result.visits.push(g.visits)
        if (g.from<result.from) result.from=g.from
        if (g.to>result.to) result.to=g.to
    }
    return result
}
function points(snapshots, selected, key, bucket) {
    const result = []
    snapshots.filter(function(s) { return selected.indexOf(s.id) >= 0 }).forEach(function(s) {
        const g = key === "*" ? combined(s.trend_series || []) : (s.trend_series || []).find(function(item) { return item.key === key })
        const period = (s.report || {}).period
        const weekly = period && isFinite(day(period.from)) && day(period.to)-day(period.from) === 6*86400000
        const counts = g ? g.buckets || [] : []
        const assessed = g && g.decisions > 0 && g.from && g.to && !g.undated
            && counts.length === 4 && counts.every(function(n) { return Number.isFinite(n) && n >= 0 })
            && counts.reduce(function(a,b) { return a+b },0) === g.decisions
        if (!weekly && !assessed) return
        const from = weekly ? period.from : g.from, to = weekly ? period.to : g.to
        const time = day(weekly ? from : to)
        if (!isFinite(time)) return
        result.push({id:s.id, groupId:s.group_id, name:s.group_name, label:s.label,
            from:from, to:to, time:time, weekly:!!weekly,
            unfinished:!!(weekly && period.unfinished),
            games:assessed ? g.games : 0, complete:assessed ? g.complete : 0,
            partial:assessed ? Math.max(0,g.games-g.complete) : 0,
            total:weekly ? period.games : g.games,
            visits:assessed ? (Array.isArray(g.visits) ? g.visits : [g.visits]) : [],
            moves:assessed ? g.decisions : 0, ids:assessed ? g.game_ids : [],
            percent:assessed ? 100*counts[bucket]/g.decisions : null,
            pending:(s.report.pending || []).length, excluded:s.report.excluded || 0})

    })
    result.sort(function(a, b) { return a.time - b.time || a.id - b.id })
    result.forEach(function(p,i) {
        const previous = result[i-1]
        p.connect = i > 0 && p.percent !== null && previous.percent !== null
            && (p.weekly || previous.weekly ? p.weekly && previous.weekly && p.time-previous.time === 7*86400000 : true)
    })
    return result
}
function day(value) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value || "")) return NaN
    const time = Date.parse(value+"T00:00:00Z")
    return isFinite(time) && new Date(time).toISOString().slice(0,10) === value ? time : NaN
}
// Place readable ticks on period boundaries; thin labels on long timelines.
function ticks(start, end, width, weekly) {
    if (!isFinite(start) || !isFinite(end) || end < start) return []
    const capacity = Math.max(2,Math.floor(width/105))
    const unit = (weekly ? 7 : 1)*86400000
    const step = Math.max(1,Math.ceil((end-start)/unit/Math.max(1,capacity-1)))*unit
    const result = []
    for (let time=start; time<=end; time+=step)
        result.push({time:time,label:new Date(time).toISOString().slice(0,10)})
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
