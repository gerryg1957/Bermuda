function day(value) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value || "")) return NaN
    const time = Date.parse(value + "T00:00:00Z")
    return isFinite(time) && new Date(time).toISOString().slice(0,10) === value ? time : NaN
}
function date(time) { return new Date(time).toISOString().slice(0,10) }
function games(report) {
    let result = []
    ;(report.groups || []).forEach(function(g) {
        (g.analysed || []).forEach(function(a) { result.push(Object.assign({},a,{key:g.key})) })
    })
    result = result.concat((report.pending || []).map(function(g) { return Object.assign({},g,{key:null,decisions:0}) }))
    return result.sort(function(a,b) { return String(a.date || "").localeCompare(String(b.date || "")) || a.game_id-b.game_id })
}
function startForMonths(today, months) {
    const end = new Date(day(today)), target = new Date(Date.UTC(end.getUTCFullYear(),end.getUTCMonth()-months,1))
    const last = new Date(Date.UTC(target.getUTCFullYear(),target.getUTCMonth()+1,0)).getUTCDate()
    target.setUTCDate(Math.min(end.getUTCDate(),last))
    return date(target.getTime())
}
// settings_key is the nine-field provenance tuple from personal_report.rs.
// Only komi is omitted: each move's loss is still evaluated at its game's komi.
function comparableKey(key) {
    try {
        const parts = JSON.parse(key)
        if (Array.isArray(parts) && parts.length === 9) {
            parts[7] = null
            return JSON.stringify(parts)
        }
    } catch (e) {}
    return key // Unknown formats must never be silently combined.
}
function assessed(game) {
    const counts = game.buckets || [], n = game.decisions || 0
    return n > 0 && counts.length === 4 && counts.every(function(v) { return Number.isFinite(v) && v >= 0 })
        && counts.reduce(function(a,b) { return a+b },0) === n
}
function settings(report, from, to) {
    const result = [], byKey = {}, start = from ? day(from) : -Infinity, end = to ? day(to) : Infinity
    ;(report.groups || []).forEach(function(g) {
        const key = comparableKey(g.key)
        let option = byKey[key]
        if (!option) {
            option = {key:key,keys:[],visits:g.visits,rules:g.rules,board_size:g.board_size,games:0}
            byKey[key] = option; result.push(option)
        }
        option.keys.push(g.key)
        option.games += (g.analysed || []).filter(function(a) {
            const t = day(a.date)
            return isFinite(t) && t >= start && t <= end && assessed(a)
        }).length
    })
    return result.sort(function(a,b) { return b.visits-a.visits || String(a.rules).localeCompare(String(b.rules)) || String(a.key).localeCompare(String(b.key)) })
}
function prepare(report, key, bucket, from, to, grouping, blockSize) {
    const keys = Array.isArray(key) ? key : [key]
    const matches = function(g) { return keys.indexOf(g.key) >= 0 }
    const all = games(report), start = from ? day(from) : -Infinity, end = to ? day(to) : Infinity
    if (isNaN(start) || isNaN(end) || start > end) return {error:"Enter valid dates, with From no later than To.",points:[]}
    const dated = all.filter(function(g) { const t=day(g.date); return isFinite(t) && t>=start && t<=end })
    const contributing = dated.filter(function(g) { return matches(g) && assessed(g) })
    // Calendar bins retain empty internal periods as gaps. Game-based views
    // contain contributing games only, with the exclusions counted separately.
    const input = grouping === "each" || grouping === "games" ? contributing : dated
    const bins = [], byKey = {}
    input.forEach(function(g,i) {
        const time = day(g.date), d = new Date(time)
        let name, binKey, t = time
        if (grouping === "week") { t = time - ((d.getUTCDay()+6)%7)*86400000; name = date(t) }
        else if (grouping === "month") { t = Date.UTC(d.getUTCFullYear(),d.getUTCMonth(),1); name = date(t).slice(0,7) }
        else if (grouping === "each") { name = g.title; binKey = "game:"+g.game_id }
        else { name = "Block " + (Math.floor(i/Math.max(1,blockSize))+1) }
        binKey = binKey || name
        let bin = byKey[binKey]
        if (!bin) { bin = {name:name,time:t,from:g.date,to:g.date,games:[],moves:0,count:0,complete:0,partial:0,pending:0,other:0}; byKey[binKey]=bin; bins.push(bin) }
        bin.to=g.date; bin.games.push(g)
        if (!g.key) bin.pending++
        else if (!matches(g)) bin.other++
        else {
            if (g.complete) bin.complete++; else bin.partial++
            if (assessed(g)) {bin.moves+=g.decisions;bin.count+=g.buckets[bucket]}
        }
    })
    // Fit the displayed timeline to evidence for the selected analysis.
    const first = bins.findIndex(function(b) { return b.moves > 0 })
    let last = bins.length-1
    while (last >= 0 && !bins[last].moves) last--
    const points = first < 0 ? [] : bins.slice(first,last+1)
    points.forEach(function(b,i) {
        b.percent=b.moves ? 100*b.count/b.moves : null
        if (grouping === "games") b.time=day(b.to)
        b.order = i
        b.connect=i>0 && b.percent!==null && points[i-1].percent!==null
            && (grouping==="games" || grouping==="each" || (grouping==="week" ? b.time-points[i-1].time===7*86400000
                : new Date(b.time).getUTCFullYear()*12+new Date(b.time).getUTCMonth()
                  -new Date(points[i-1].time).getUTCFullYear()*12-new Date(points[i-1].time).getUTCMonth()===1))
    })
    return {error:"",points:points,undated:all.filter(function(g) {return !isFinite(day(g.date))}).length,
        games:dated.length,contributing:contributing.length,
        pending:dated.filter(function(g) {return !g.key}).length,
        other:dated.filter(function(g) {return g.key && !matches(g)}).length,
        unassessed:dated.filter(function(g) {return matches(g) && !assessed(g)}).length}
}
