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
