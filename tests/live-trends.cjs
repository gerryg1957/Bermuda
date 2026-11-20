const fs=require('fs'), vm=require('vm'), assert=require('assert');
const t={}; vm.createContext(t); vm.runInContext(fs.readFileSync('bermuda-qt/src/qml/LiveTrends.js','utf8'),t);
const game=(id,date,buckets,complete=true)=>({game_id:id,date,buckets,decisions:buckets.reduce((a,b)=>a+b,0),complete,title:'Me — Opponent'});
const report={groups:[{key:'a',analysed:[game(1,'2026-11-02',[1,0,0,0]),game(2,'2026-11-03',[0,9,0,0],false),game(3,'2026-11-16',[2,0,0,0]),game(4,'unknown',[1,0,0,0])]}, {key:'b',analysed:[game(5,'2026-11-03',[100,0,0,0])]}],pending:[{game_id:6,date:'2026-11-09'}]};
const p=t.prepare(report,'a',0,'','','week',20);
assert.equal(p.points[0].percent,10); // Weighted by moves, not average game percentage.
assert.equal(p.points[0].other,1);assert.equal(p.points[0].partial,1);
assert.equal(p.points[1].percent,null);assert.equal(p.points[2].connect,false);
assert.equal(p.undated,1);assert.equal(p.games,5);
assert.equal(t.prepare(report,'a',0,'2026-11-16','2026-11-16','week',20).games,1);
assert.ok(t.prepare(report,'a',0,'2026-02-30','','week',20).error);
assert.ok(t.prepare(report,'a',0,'2026-11-20','2026-11-01','week',20).error);
assert.equal(t.startForMonths('2026-03-31',1),'2026-02-28');
assert.equal(t.startForMonths('2024-03-31',1),'2024-02-29');
const blocks=t.prepare(report,'a',0,'','','games',2).points;
assert.equal(blocks.length,2);assert.equal(blocks[0].games.length,2);assert.equal(blocks[1].games.length,1);
assert.equal(t.prepare({groups:[{key:'a',analysed:[game(1,'2026-11-01',[1,0,0,0]),game(2,'2026-12-01',[1,0,0,0])]}]},'a',0,'','','month',20).points[1].connect,true);
console.log('PASS: live trends weighting, settings, missing evidence, dates and game blocks');

// Only komi may be merged; backend, search depth and rules remain distinct.
const key=(komi,visits=1000,engine='gpu',rules='japanese')=>JSON.stringify([1,engine,'network','config','Black',visits,rules,komi,19]);
const sample={groups:[
 {key:key(-5.5),visits:1000,rules:'japanese',board_size:19,analysed:[game(1,'2026-11-01',[1,0,0,0])]},
 {key:key(6.5),visits:1000,rules:'japanese',board_size:19,analysed:[game(2,'2026-11-01',[0,9,0,0])]},
 {key:key(0.5,200),visits:200,rules:'japanese',board_size:19,analysed:[game(3,'2026-10-01',[99,0,0,0])]},
 {key:key(0.5,1000,'other'),visits:1000,rules:'japanese',board_size:19,analysed:[game(4,'2026-11-08',[5,0,0,0])]},
 {key:key(0.5,1000,'gpu','chinese'),visits:1000,rules:'chinese',board_size:19,analysed:[game(5,'2026-11-09',[2,0,0,0])]}
],pending:[{game_id:6,date:'2026-09-01'}]};
const choices=t.settings(sample,'','');assert.equal(choices.length,4);
const combined=choices.find(s=>s.keys.length===2);assert.equal(combined.games,2);
const combinedPoints=t.prepare(sample,combined.keys,0,'','','week',20);
assert.equal(combinedPoints.points.length,1);assert.equal(combinedPoints.points[0].percent,10);
assert.equal(combinedPoints.contributing,2);assert.equal(combinedPoints.games,6);
assert.equal(combinedPoints.other,3);assert.equal(combinedPoints.pending,1);
const each=t.prepare(sample,combined.keys,0,'','','each',20).points;
assert.equal(each.length,2);assert.equal(each[0].time,each[1].time);
assert.equal(each[0].order,0);assert.equal(each[1].order,1);
assert.equal(each[0].percent,100);assert.equal(each[1].percent,0);
assert.equal(t.settings(sample,'2026-11-02','').find(s=>s.keys.length===2).games,0);
assert.equal(t.prepare(sample,combined.keys,0,'2026-11-02','','each',20).points.length,0);
assert.notEqual(t.comparableKey('unknown1'),t.comparableKey('unknown2'));
console.log('PASS: komi pooling, engine/depth separation, each-game ordering and contributing counts');

// Seven-day report boundaries start on the first game, not calendar Monday.
const anchoredReport={groups:[{key:'a',analysed:[
 game(1,'2026-10-01',[1,0,0,0]), game(2,'2026-10-08',[2,0,0,0]),
 game(3,'2026-10-14',[0,3,0,0]), game(4,'2026-10-15',[4,0,0,0]),
 game(5,'2026-10-29',[1,0,0,0])
]}]};
const anchored=t.prepare(anchoredReport,'a',0,'','','week',20,'2026-10-08');
assert.equal(anchored.games,4);
assert.equal(anchored.points[0].name,'2026-10-08 – 2026-10-14');
assert.equal(anchored.points[0].percent,40);
assert.equal(anchored.points[1].connect,true);
assert.equal(anchored.points[2].percent,null);
assert.equal(anchored.points[3].connect,false); // Empty week is not a zero score.
assert.equal(t.date(anchored.points[1].time),'2026-10-15');
console.log('PASS: report-anchored weeks, exact boundaries and empty-week gaps');

const current=t.prepare(anchoredReport,'a',0,'','','week',20,'2026-10-08','2026-10-30');
assert.equal(current.points[3].unfinished,true);
assert.equal(current.points[0].unfinished,false);
assert.equal(current.points[2].from,'2026-10-22');
assert.equal(current.points[2].games.length,0);
console.log('PASS: explicit empty weeks and unfinished live period');

const allChoice=t.allOption(sample,'','')[0];
assert.equal(allChoice.key,'*');
assert.equal(allChoice.keys.length,5);
const allDepths=t.prepare(sample,allChoice.keys,0,'','','each',20);
assert.equal(allDepths.contributing,5);
assert.equal(allDepths.other,0);
assert.equal(allDepths.pending,1);
const allWeek=t.prepare(sample,allChoice.keys,0,'2026-11-01','2026-11-01','week',20);
assert.equal(allWeek.points[0].percent,10); // 1 of 10 moves, not 50% average of games.
assert.deepEqual(Array.from(allWeek.points[0].visits),[1000]);
assert.equal(t.allOption({},'','').length,0);
console.log('PASS: opt-in all analysis uses all keys, keeps pending separate and weights moves');
