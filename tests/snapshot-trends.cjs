// Run with node tests/snapshot-trends.cjs. Exercises the same JS used by QML.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict');
const t = {}; vm.createContext(t); vm.runInContext(fs.readFileSync('bermuda-qt/src/qml/SnapshotTrends.js','utf8'),t);
function report(id,group,date,ids,buckets,key='cpu') {
 return {id,group_id:group,group_name:'Group '+group,label:'Report '+id,report:{pending:[],excluded:0},
 trend_series:[{key,visits:200,rules:'japanese',komi:6.5,board_size:19,
 decisions:buckets.reduce((a,b)=>a+b,0),buckets,games:ids.length,complete:ids.length,
 from:date,to:date,undated:0,game_ids:ids}]};
}
const a=report(1,1,'2026-10-01',[1],[50,30,10,10]);
const b=report(2,2,'2026-11-01',[2,3],[160,20,10,10]);
const c=report(3,1,'2026-10-01',[1],[60,20,10,10],'gpu');
let p=t.points([b,a],[1,2],'cpu',0);
assert.equal(p[0].percent,50); assert.equal(p[1].percent,80); // normalise unequal denominators
assert.equal(t.overlaps(p),false); assert.equal(p[0].id,1); // game dates, not list/save order
assert.deepEqual(Array.from(t.choose([a,b,c],[1,2],3,true)),[2,3]); // one snapshot per group
assert.equal(t.settings([a,b,c],[1,2,3]).length,2); // different engines never combined
assert.equal(t.points([a,b,c],[1,2,3],'gpu',0).length,1);
const overlap=report(4,4,'2026-11-02',[1,4],[40,30,20,10]);
assert.equal(t.overlaps(t.points([a,overlap],[1,4],'cpu',0)),true);
const empty=report(5,5,'2026-11-03',[5],[0,0,0,0]);
assert.equal(t.points([empty],[5],'cpu',0).length,0);
const undated=report(6,6,'2026-11-03',[6],[1,0,0,0]); undated.trend_series[0].undated=1;
assert.equal(t.points([undated],[6],'cpu',0).length,0);
console.log('PASS: trend percentages, chronology, report selection, settings, overlap and missing evidence');

const w1=report(10,10,'2026-10-12',[10],[1,1,0,0]);
w1.report.period={from:'2026-10-08',to:'2026-10-14',games:1,unfinished:false};
const w2=report(11,11,'2026-10-17',[11],[0,0,0,0]);
w2.report.period={from:'2026-10-15',to:'2026-10-21',games:1,unfinished:false};
w2.report.pending=[{game_id:11}];
const w3=report(12,12,'2026-10-24',[12],[1,0,0,0]);
w3.report.period={from:'2026-10-22',to:'2026-10-28',games:1,unfinished:true};
p=t.points([w3,w2,w1],[10,11,12],'cpu',0);
assert.equal(p[0].time,Date.parse('2026-10-08T00:00:00Z'));
assert.equal(p[1].percent,null);
assert.equal(p[1].pending,1);
assert.equal(p[2].connect,false);
assert.equal(p[2].unfinished,true);
assert.equal(p[0].total,1);
assert.equal(t.points([w1,w3],[10,12],'cpu',0)[1].connect,false);
assert.equal(t.points([w1,w2],[10,11],'other',0)[0].percent,null);
w2.trend_series[0].decisions=2;w2.trend_series[0].buckets=[1,1,0,0];
assert.equal(t.points([w1,w2],[10,11],'cpu',0)[1].connect,true);
const ticks=t.ticks(p[0].time,p[2].time,800,true);
assert.equal(ticks.length,3);
assert.equal(ticks[1].label,'2026-10-15');
assert.ok(t.ticks(p[0].time,p[0].time+52*7*86400000,500,true).length<=5);
console.log('PASS: fixed weekly dates, missing evidence, adjacency, unfinished snapshots and readable ticks');

const mixed=report(20,20,'2026-11-05',[20],[1,0,0,0],'cpu');
const newer=report(21,21,'2026-11-05',[21],[0,9,0,0],'gpu');
newer.trend_series[0].visits=1000;
mixed.trend_series.push(newer.trend_series[0]);
mixed.report.period={from:'2026-11-05',to:'2026-11-11',games:2,unfinished:false};
const allPoint=t.points([mixed],[20],'*',0)[0];
assert.equal(allPoint.percent,10);
assert.equal(allPoint.games,2);
assert.equal(allPoint.moves,10);
assert.deepEqual(Array.from(allPoint.visits),[200,1000]);
assert.equal(t.points([mixed],[20],'cpu',0)[0].percent,100); // specific setting unchanged
assert.equal(t.points([mixed],[20],'gpu',0)[0].percent,0);
const duplicate=structuredClone(mixed);
duplicate.trend_series[1].game_ids=[20];
assert.equal(t.points([duplicate],[20],'*',0)[0].percent,null);
const corrupt=structuredClone(mixed);
corrupt.trend_series[1].buckets=[0,-9,0,0];
assert.equal(t.points([corrupt],[20],'*',0)[0].percent,100); // invalid evidence excluded
console.log('PASS: mixed-depth/engine snapshots, weighted buckets and duplicate/corrupt guards');
