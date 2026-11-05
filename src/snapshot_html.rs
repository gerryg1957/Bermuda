//! Self-contained, offline report. User/engine text is always HTML-escaped.
use crate::{Board, Colour, game_analysis::GameAnalysis, personal_snapshots::Snapshot};
use anyhow::Result;
use std::fmt::Write;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn download(text: &str) -> String {
    text.as_bytes()
        .iter()
        .map(|b| format!("%{b:02X}"))
        .collect()
}
fn text(v: &serde_json::Value) -> String {
    escape(v.as_str().unwrap_or(""))
}
fn number(v: &serde_json::Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}
// Gregorian UTC date, no local timezone dependency in a preserved report.
fn timestamp(seconds: u64) -> String {
    let days = (seconds / 86400) as i64 + 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}
fn colour(c: Option<Colour>) -> &'static str {
    match c {
        Some(Colour::Black) => "Black",
        Some(Colour::White) => "White",
        None => "Unknown / ambiguous",
    }
}

pub(crate) fn render(id: i64, s: &Snapshot) -> Result<String> {
    let mut out = String::from(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Bermuda · preserved report</title><style>
body{font:16px/1.5 system-ui,sans-serif;color:#202e35;max-width:1050px;margin:32px auto;padding:0 24px}h1{font-size:30px;margin-bottom:4px}h2{margin-top:30px}h3{margin-bottom:6px}a{color:#155f77}table{border-collapse:collapse;width:100%;margin:16px 0}th,td{text-align:left;vertical-align:top;border-bottom:1px solid #cbd3d6;padding:7px}th{background:#f0f4f4}.muted{color:#52616a}.coverage{padding:12px;border:1px solid #91b6af;background:#f1f8f6}.toolbar{position:sticky;top:0;padding:12px;background:white;border-bottom:1px solid #aaa;display:flex;gap:18px;align-items:center}button{padding:8px 14px;font:inherit;cursor:pointer}.game{break-before:page}.diagram{display:inline-block;vertical-align:top;width:31%;margin-right:2%;break-inside:avoid}.diagram svg{width:100%;height:auto}.graph{width:100%;height:auto}svg text{font:12px sans-serif;fill:#263238}pre{white-space:pre-wrap;overflow-wrap:anywhere;font-size:12px}small{font-size:13px}.decision-table{font-size:14px}.decision-table td{padding:4px 7px}.game-pages{display:none}body.show-games .game-pages{display:block}.details{display:none}body.show-details .details{display:block}.cost-chart,.cards{break-inside:avoid}.chart{width:100%;height:auto;display:block}.cards{display:flex;gap:18px;align-items:flex-start}.card{flex:1;max-width:32%;min-width:0;break-inside:avoid}.card svg{width:100%;height:auto}.card p{font-size:14px;margin:6px 0}.card h3{font-size:16px;margin:0 0 6px}.intro{margin:8px 0}.subtitle{font-size:19px;font-weight:600}.next{border-top:1px solid #cbd3d6;margin-top:22px;padding-top:12px}.legend{display:flex;flex-wrap:wrap;gap:8px 22px;margin:8px 0}.legend span{white-space:nowrap}.key{display:inline-block;width:12px;height:12px;border:1px solid #6d777b;margin-right:6px}.visual-set h2{margin:20px 0 6px}.visual-set p{margin:6px 0}.details{break-before:page}@media(max-width:600px){.toolbar{position:static;flex-wrap:wrap}.cards{flex-wrap:wrap}.card{max-width:none;flex-basis:100%}}@page{size:A4;margin:16mm}@media print{body{font-size:10pt;line-height:1.3;margin:0;padding:0;max-width:none}h1{margin-top:0}p{margin:6px 0}.toolbar,.no-print{display:none!important}h1{font-size:21pt}h2{font-size:16pt}th,td{padding:4px}thead{display:table-header-group}tr{break-inside:avoid}a{color:inherit;text-decoration:none}.diagram{width:31%}pre{font-size:8pt}.decision-table{font-size:9pt}.coverage{background:white}.cost-chart,.cards{break-inside:avoid}.card p{font-size:9pt}.card h3{font-size:10pt}.visual-set h2{font-size:14pt}.card{max-width:32%}.intro{font-size:10pt}}
</style></head><body><div class="toolbar"><button onclick="window.print()">Print / Save as PDF</button><label><input type="checkbox" onchange="document.body.classList.toggle('show-games',this.checked)"> Game pages and engine details</label><label><input type="checkbox" onchange="document.body.classList.toggle('show-details',this.checked)"> Detailed tables</label></div>"#,
    );
    write!(out,"<h1>{}</h1><div class=subtitle>{}</div><p class=intro>A fixed copy of your saved analysis. Later analysis will not change this report.</p><p class=muted>Saved {} · snapshot #{id}</p>",escape(&s.group.name),escape(&s.label),timestamp(s.created_at)).unwrap();
    write!(out, "<style>@page{{@bottom-left{{content:\"Bermuda · snapshot #{id}\";font-size:8pt}}@bottom-right{{content:counter(page);font-size:8pt}}}}</style>").unwrap();
    let r = &s.report;
    out.push_str(&visual_summary(s)?);
    out.push_str("<p class=\"next no-print\"><strong>What next?</strong> Explore these positions in Bermuda. Different groups with comparable analysis can help you investigate changes in play. Deeper analysis of the same games improves the evidence. A recurring weakness still needs investigation.</p><p class=\"muted no-print\">Bermuda organises evidence; KataGo evaluates; you investigate.</p><section class=details><h2>Detailed tables</h2>");
    out.push_str("<p>This is a fixed copy of saved evidence, not a live report. New imports, name edits and later analysis do not change it. Names and your colour are those saved with this group. Partial games contribute only assessed moves.</p><h2>Games in this group</h2><table><thead><tr><th>Date</th><th>Black</th><th>White</th><th>You</th><th>Result</th><th>Coverage</th></tr></thead><tbody>");
    for (m, run) in s.group.members.iter().zip(&s.runs) {
        let coverage = if m.colour.is_none() {
            format!("Excluded: {}", m.identity_status)
        } else {
            match run {
                Some(run) => format!(
                    "{} · {}/{} positions",
                    if run.complete() {
                        "Complete"
                    } else {
                        "Partial"
                    },
                    run.results.len(),
                    run.total()
                ),
                None => "Awaiting analysis".into(),
            }
        };
        write!(
            out,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape(&m.date),
            escape(&m.black),
            escape(&m.white),
            colour(m.colour),
            escape(&m.result),
            escape(&coverage)
        )
        .unwrap();
    }
    out.push_str("</tbody></table><h2>Saved analysis</h2><p>Settings are kept in separate sets. These are estimates for investigation, not a playing-strength grade or a verdict on why a game was lost.</p>");
    if let Some(groups) = r["groups"].as_array() {
        if groups.is_empty() {
            out.push_str("<p>No assessed personal decisions were available when this snapshot was saved.</p>");
        }
        for (i, g) in groups.iter().enumerate() {
            write!(out,"<h3>Analysis set {} · {} games</h3><p>{} visits per position · {} rules · komi {}<br>{} of your moves assessed</p><table><thead><tr><th>Estimated points lost</th><th>Moves</th><th>Share of assessed moves</th></tr></thead><tbody>",i+1,g["games"],g["visits"],text(&g["rules"]),g["komi"],g["decisions"]).unwrap();
            for (b, label) in ["Below 1 (including estimated gains)", "1–3", "3–5", "5+"]
                .iter()
                .enumerate()
            {
                write!(
                    out,
                    "<tr><td>{label}</td><td>{}</td><td>{:.1}%</td></tr>",
                    g["buckets"][b],
                    number(&g["buckets"][b]) * 100.0 / number(&g["decisions"]).max(1.0)
                )
                .unwrap();
            }
            out.push_str("</tbody></table><h3>Changes in winning chances</h3><p>Up to 20 largest saved drops of at least 5 percentage points, from your perspective.</p><table><thead><tr><th>Game</th><th>Move / colour</th><th>Your win estimate</th></tr></thead><tbody>");
            if let Some(examples) = g["win_examples"].as_array() {
                for d in examples {
                    write!(
                        out,
                        "<tr><td>{}<br>{}</td><td>{} · {}</td><td>{:.1}% → {:.1}%{}</td></tr>",
                        text(&d["title"]),
                        text(&d["date"]),
                        d["move"],
                        text(&d["colour"]),
                        number(&d["win_before"]) * 100.0,
                        number(&d["win_after"]) * 100.0,
                        if d["already_unlikely"] == true {
                            " · already ≤5%"
                        } else {
                            ""
                        }
                    )
                    .unwrap();
                }
            }
            out.push_str("</tbody></table><h3>Large point losses</h3><p>Up to 20 largest saved estimates of at least one point. A large loss can occur in a game already unlikely to be won.</p><table><thead><tr><th>Game</th><th>Move / colour</th><th>Points lost (estimate)</th></tr></thead><tbody>");
            if let Some(examples) = g["examples"].as_array() {
                for d in examples {
                    write!(
                        out,
                        "<tr><td>{}<br>{}</td><td>{} · {}</td><td>{:.1}{}</td></tr>",
                        text(&d["title"]),
                        text(&d["date"]),
                        d["move"],
                        text(&d["colour"]),
                        number(&d["loss"]),
                        if d["already_unlikely"] == true {
                            " · win estimate already ≤5%"
                        } else {
                            ""
                        }
                    )
                    .unwrap();
                }
            }
            out.push_str("</tbody></table>");
        }
    }
    out.push_str("</section><div class=game-pages>");
    for (index, (m, run)) in s.group.members.iter().zip(&s.runs).enumerate() {
        write!(out,"<section class=game><h2>{} — {}</h2><p>{} · You: {} · Recorded result: {}</p><p class=no-print><a download=\"game-{}.sgf\" href=\"data:application/x-go-sgf,{}\">Download preserved game (SGF)</a></p>",escape(&m.black),escape(&m.white),escape(&m.date),colour(m.colour),escape(&m.result),m.game_id,download(&m.sgf)).unwrap();
        let Some(run) = run else {
            out.push_str("<p>No analysis saved in this snapshot.</p></section>");
            continue;
        };
        write!(
            out,
            "<p>{} · {}/{} positions · {} visits · {} rules · komi {}</p>",
            if run.complete() {
                "Complete"
            } else {
                "Partial"
            },
            run.results.len(),
            run.total(),
            run.request.max_visits,
            escape(&run.request.rules),
            run.request.komi
        )
        .unwrap();
        out.push_str(&score_graph(run));
        out.push_str("<p class=muted>Score: Black positive (points). X-axis: move number. The line stops where saved analysis ends.</p>");
        if m.colour.is_some() {
            let decisions = &s.decisions[index];
            let mut important: Vec<_> = decisions
                .iter()
                .filter(|d| number(&d["win_loss"]) >= 5.0)
                .collect();
            important.sort_by(|a, b| number(&b["win_loss"]).total_cmp(&number(&a["win_loss"])));
            important.truncate(3);
            if !important.is_empty() {
                out.push_str("<h3>Positions to revisit</h3><p>Before up to three of your largest drops in winning chance. The ring marks your recorded move, not an engine recommendation.</p>");
                let record =
                    crate::extract_main_variation(&crate::parse_collection(m.sgf.as_bytes())?)?;
                let positions = crate::replay_positions(&record)?;
                for d in important {
                    let turn = d["move"].as_u64().unwrap_or(1) as usize;
                    if let Some(state) = positions.get(turn.saturating_sub(1)) {
                        write!(out,"<div class=diagram><strong>Before move {turn}</strong>{}<p>Your win estimate {:.1}% → {:.1}%</p></div>",board_svg(&state.board,record.moves.get(turn.saturating_sub(1)).and_then(|m|m.point)),number(&d["win_before"])*100.0,number(&d["win_after"])*100.0).unwrap();
                    }
                }
            }
            out.push_str("<h3>Your assessed decisions</h3><table class=decision-table><thead><tr><th>Move</th><th>Points lost (estimate)</th><th>Your win estimate before → after</th></tr></thead><tbody>");
            for d in decisions {
                write!(
                    out,
                    "<tr><td>{}</td><td>{:.1}</td><td>{:.1}% → {:.1}%{}</td></tr>",
                    d["move"],
                    number(&d["estimated_loss"]),
                    number(&d["win_before"]) * 100.0,
                    number(&d["win_after"]) * 100.0,
                    if d["already_unlikely"] == true {
                        " · already ≤5%"
                    } else {
                        ""
                    }
                )
                .unwrap();
            }
            out.push_str("</tbody></table>");
        } else {
            out.push_str("<p>This game is excluded from personal statistics because your colour was not identified when the group was saved.</p>");
        }
        write!(out,"<h3>Analysis record</h3><p>Run: {} · original analysis created {}<br>Bermuda {} · policy {}</p><pre>Engine SHA-256: {}\nNetwork SHA-256: {}\nConfiguration SHA-256: {}\nScore perspective: {}</pre><p class=no-print><a download=\"analysis-{}.json\" href=\"data:application/json,{}\">Download preserved analysis, including candidates (JSON)</a></p></section>",escape(&run.run_id),timestamp(run.provenance.created_at),escape(&run.provenance.bermuda_version),run.provenance.policy,escape(&run.provenance.engine_sha256),escape(&run.provenance.network_sha256),escape(&run.provenance.config_sha256),escape(&run.provenance.score_perspective),m.game_id,download(&serde_json::to_string_pretty(run)?)).unwrap();
    }
    out.push_str("</div></body></html>");
    Ok(out)
}

// The archive is unchanged: charts use the same frozen counts as the tables.
const COVERAGE_COLOURS: [&str; 4] = ["#32877d", "#6b93ae", "#b58a45", "#9c9c9c"];
const LOSS_COLOURS: [&str; 4] = ["#32877d", "#6b93ae", "#b58a45", "#ab6262"];
fn coverage_chart(counts: [u64; 4], total: usize) -> String {
    let labels = [
        "Complete",
        "Partial",
        "Awaiting analysis",
        "Unidentified / ambiguous",
    ];
    let description = labels
        .iter()
        .zip(counts)
        .map(|(label, count)| format!("{label}: {count}"))
        .collect::<Vec<_>>()
        .join("; ");
    let mut out = format!(
        r##"<svg class="chart" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 28" role="img" aria-label="{}"><rect width="760" height="28" fill="#edf0f2"/>"##,
        escape(&description)
    );
    let mut x = 0.0;
    for (i, count) in counts.iter().enumerate() {
        let width = *count as f64 / total.max(1) as f64 * 760.0;
        write!(
            out,
            r#"<rect x="{x:.3}" y="0" width="{width:.3}" height="28" fill="{}"/>"#,
            COVERAGE_COLOURS[i]
        )
        .unwrap();
        x += width;
    }
    out.push_str("</svg><div class=legend>");
    for (i, count) in counts.iter().enumerate() {
        write!(
            out,
            r#"<span><i class="key" style="background:{}"></i><strong>{count}</strong> {}</span>"#,
            COVERAGE_COLOURS[i], labels[i]
        )
        .unwrap();
    }
    out.push_str("</div>");
    out
}
fn loss_chart(buckets: [u64; 4], total: u64) -> String {
    if total == 0 {
        return "<p>No personal moves assessed in this set yet.</p>".into();
    }
    let labels = ["Below 1 point", "1–3 points", "3–5 points", "5+ points"];
    let description = labels
        .iter()
        .zip(buckets)
        .map(|(label, count)| {
            format!(
                "{label}: {count} moves, {:.1}%",
                count as f64 * 100.0 / total as f64
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let mut out = format!(
        r#"<svg class="chart" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 169" role="img" aria-label="{}">"#,
        escape(&description)
    );
    for (i, count) in buckets.iter().enumerate() {
        let y = 8 + i * 32;
        let pct = *count as f64 * 100.0 / total as f64;
        write!(out,r##"<text x="0" y="{}">{}</text><rect x="132" y="{y}" width="448" height="20" rx="3" fill="#edf0f2"/><rect x="132" y="{y}" width="{:.3}" height="20" rx="3" fill="{}"/><text x="598" y="{}">{pct:.1}% · {count} moves</text>"##,y+15,labels[i],pct*4.48,LOSS_COLOURS[i],y+15).unwrap();
    }
    for (pct, label) in [(0, "0%"), (50, "50%"), (100, "100%")] {
        write!(
            out,
            r#"<text x="{}" y="158" text-anchor="middle">{label}</text>"#,
            132.0 + pct as f64 * 4.48
        )
        .unwrap();
    }
    out.push_str("</svg>");
    out
}
/// Select from one compatible settings set, favouring distinct games rather
/// than filling the page with several moves from the same fight.
fn highlighted(g: &serde_json::Value) -> Vec<&serde_json::Value> {
    let wins = g["win_examples"].as_array().filter(|a| !a.is_empty());
    let examples = wins.or_else(|| g["examples"].as_array());
    let mut seen = std::collections::BTreeSet::new();
    examples
        .into_iter()
        .flatten()
        .filter(|d| seen.insert(d["game_id"].as_i64()))
        .take(3)
        .collect()
}
fn visual_summary(s: &Snapshot) -> Result<String> {
    let r = &s.report;
    let mut out = format!(
        "<h2>Analysis coverage · {} games</h2>",
        s.group.members.len()
    );
    out.push_str(&coverage_chart(
        [
            r["complete"].as_u64().unwrap_or(0),
            r["partial"].as_u64().unwrap_or(0),
            r["pending"].as_array().map_or(0, |p| p.len() as u64),
            r["excluded"].as_u64().unwrap_or(0),
        ],
        s.group.members.len(),
    ));
    let groups = r["groups"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    if groups.is_empty() {
        out.push_str("<p>No saved analysis to chart yet. Analyse these games in Bermuda, then preserve a new snapshot. This copy will remain unchanged.</p>");
    }
    if groups.len() > 1 {
        out.push_str("<p>Analysis settings differ. Each set below has its own denominator; they are not combined.</p>");
    }
    for (i, g) in groups.iter().enumerate() {
        out.push_str(
            "<section class=visual-set><div class=cost-chart><h2>Estimated cost of my moves",
        );
        if groups.len() > 1 {
            write!(out, " · set {}", i + 1).unwrap();
        }
        out.push_str("</h2>");
        write!(
            out,
            "<p>{} assessed moves · {} games · {} visits · {} rules · komi {}</p>",
            g["decisions"],
            g["games"],
            g["visits"],
            text(&g["rules"]),
            g["komi"]
        )
        .unwrap();
        out.push_str(&loss_chart(
            std::array::from_fn(|i| g["buckets"][i].as_u64().unwrap_or(0)),
            g["decisions"].as_u64().unwrap_or(0),
        ));
        out.push_str("<p class=muted>Share of assessed personal moves, including partial games. Below 1 includes estimated gains. This is not a strength grade.</p></div>");
        let selected = highlighted(g);
        if !selected.is_empty() {
            let by_win = g["win_examples"].as_array().is_some_and(|a| !a.is_empty());
            out.push_str("<h2>Positions to investigate</h2>");
            out.push_str(if by_win {"<p>Largest saved win-estimate drops, with one position per game (up to three).</p>"}else{"<p>No large win-estimate drops were saved. These examples show estimated point losses instead.</p>"});
            out.push_str("<div class=cards>");
            for d in selected {
                let Some(m) = s
                    .group
                    .members
                    .iter()
                    .find(|m| Some(m.game_id) == d["game_id"].as_i64())
                else {
                    continue;
                };
                let record =
                    crate::extract_main_variation(&crate::parse_collection(m.sgf.as_bytes())?)?;
                let states = crate::replay_positions(&record)?;
                let turn = d["move"].as_u64().unwrap_or(1) as usize;
                let Some(state) = states.get(turn.saturating_sub(1)) else {
                    continue;
                };
                let point = record
                    .moves
                    .get(turn.saturating_sub(1))
                    .and_then(|mv| mv.point);
                let played = point
                    .map(|p| state.board.point_name(p))
                    .transpose()?
                    .unwrap_or_else(|| "pass".into());
                write!(out,"<article class=card><h3>{} — {}</h3><p>{} · You: {}<br>Recorded result: {}</p><strong>Before move {turn}</strong>{}<p>You played <strong>{}</strong><br>Win estimate {:.1}% → {:.1}%<br>Estimated cost: {:.1} points{}</p></article>",escape(&m.black),escape(&m.white),escape(&m.date),colour(m.colour),escape(&m.result),board_svg(&state.board,point),escape(&played),number(&d["win_before"])*100.0,number(&d["win_after"])*100.0,number(&d["loss"]),if d["already_unlikely"]==true {"<br>Already ≤5% before this move"}else{""}).unwrap();
            }
            out.push_str("</div><p class=muted>Ring: your recorded move. Examples to explore, not established weaknesses.</p>");
        } else {
            out.push_str("<p>No saved decisions above the report thresholds. That does not mean there is nothing to learn.</p>");
        }
        out.push_str("</section>");
    }
    Ok(out)
}

fn score_graph(run: &GameAnalysis) -> String {
    let max = run
        .results
        .iter()
        .map(|r| r.score_lead)
        .fold(1.0, f64::max)
        .ceil();
    let min = run
        .results
        .iter()
        .map(|r| r.score_lead)
        .fold(-1.0, f64::min)
        .floor();
    let end = run.request.moves.len().max(1) as f64;
    let x = |turn: u32| 45.0 + turn as f64 / end * 700.0;
    let y = |score: f64| 20.0 + (max - score) / (max - min) * 150.0;
    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" class="graph" viewBox="0 0 780 205" role="img" aria-label="Saved score graph, Black positive"><path d="M45 20 V170 H745 M45 {} H745" stroke="#b2bec5" fill="none"/><text x="4" y="28">{max:.0}</text><text x="4" y="170">{min:.0}</text>"##,
        y(0.0)
    );
    let step = if end <= 50.0 { 10 } else { 50 };
    let ticks: std::collections::BTreeSet<_> = (0..=run.request.moves.len())
        .step_by(step)
        .chain(std::iter::once(run.request.moves.len()))
        .collect();
    for turn in ticks {
        write!(
            s,
            r#"<text x="{}" y="195" text-anchor="middle">{turn}</text>"#,
            x(turn as u32)
        )
        .unwrap();
    }
    s.push_str(r##"<polyline fill="none" stroke="#267d9c" stroke-width="2" points=""##);
    for r in &run.results {
        write!(s, "{},{} ", x(r.turn_number), y(r.score_lead)).unwrap();
    }
    s.push_str("\"/></svg>");
    s
}
fn board_svg(board: &Board, played: Option<u16>) -> String {
    let size = u16::from(board.size());
    let end = 25 + (size - 1) * 20;
    let mut s = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" role="img" aria-label="Position before the recorded move"><rect width="100%" height="100%" fill="#fbf5e8"/>"##,
        end + 25,
        end + 25
    );
    for i in 0..size {
        let p = 25 + i * 20;
        let letter = if i < 8 {
            (b'A' + i as u8) as char
        } else {
            (b'B' + i as u8) as char
        };
        write!(s,r##"<path d="M25 {p} H{end} M{p} 25 V{end}" fill="none" stroke="#8f887b" stroke-width="1"/><text x="{p}" y="14" text-anchor="middle">{letter}</text><text x="3" y="{}">{}</text>"##,p+4,size-i).unwrap();
    }
    for p in 0..size * size {
        if let Some(c) = board.colour_at(p) {
            write!(
                s,
                r##"<circle cx="{}" cy="{}" r="9" fill="{}" stroke="#222"/>"##,
                25 + p % size * 20,
                25 + p / size * 20,
                if c == Colour::Black { "#222" } else { "white" }
            )
            .unwrap();
        }
    }
    if let Some(p) = played {
        write!(
            s,
            r##"<circle cx="{}" cy="{}" r="7" fill="none" stroke="#ba3e45" stroke-width="3"/>"##,
            25 + p % size * 20,
            25 + p / size * 20
        )
        .unwrap();
    }
    s.push_str("</svg>");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn charts_show_counts_and_use_their_own_denominators() {
        let coverage = coverage_chart([2, 1, 1, 0], 4);
        assert!(coverage.contains("Complete: 2; Partial: 1; Awaiting analysis: 1"));
        assert!(coverage.contains(r#"width="380.000""#));
        assert!(coverage.contains(r#"width="190.000""#));
        let costs = loss_chart([5, 3, 2, 0], 10);
        assert!(costs.contains("50.0% · 5 moves"));
        assert!(costs.contains("30.0% · 3 moves"));
        assert!(costs.contains(r#"width="224.000""#));
        assert!(!loss_chart([0; 4], 0).contains("NaN"));
        assert!(loss_chart([0; 4], 0).contains("No personal moves assessed"));
    }
    #[test]
    fn highlights_keep_distinct_games_and_use_point_losses_only_as_fallback() {
        let group = serde_json::json!({"win_examples":[
            {"game_id":1,"move":5},{"game_id":1,"move":7},
            {"game_id":2,"move":8},{"game_id":3,"move":9},{"game_id":4,"move":10}],
            "examples":[{"game_id":99,"move":1}]});
        let picked = highlighted(&group);
        assert_eq!(picked.len(), 3);
        assert_eq!(
            picked
                .iter()
                .map(|d| d["game_id"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let fallback = serde_json::json!({"win_examples":[],"examples":[{"game_id":99,"move":1}]});
        assert_eq!(highlighted(&fallback)[0]["game_id"], 99);
        assert!(highlighted(&serde_json::json!({})).is_empty());
    }
    #[test]
    fn date_and_text_are_stable_and_safe() {
        assert_eq!(timestamp(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(timestamp(951782400), "2000-02-29 00:00:00 UTC");
        assert_eq!(escape("<script>\"&'"), "&lt;script&gt;&quot;&amp;&#39;");
        assert_eq!(download("#<"), "%23%3C");
    }
    #[test]
    fn board_uses_sgf_top_left_and_marks_recorded_move() -> Result<()> {
        let mut b = Board::new(19)?;
        b.set_setup(Colour::Black, 0)?;
        let svg = board_svg(&b, Some(342));
        assert!(svg.contains(r#"cx="25" cy="25" r="9""#));
        assert!(svg.contains(r#"cx="25" cy="385" r="7""#));
        Ok(())
    }
}
