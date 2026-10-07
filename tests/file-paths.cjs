// Run with node tests/file-paths.cjs. Execute the actual shared QML JavaScript.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const repo = path.resolve(__dirname, '..');
const source = fs.readFileSync(path.join(repo, 'bermuda-qt/src/qml/FilePaths.js'), 'utf8');
const context = vm.createContext({URL, decodeURIComponent});
vm.runInContext(source.replace(/^\.pragma library\s*/, ''), context);
const cases = [
    ['file:///C:/Users/richard/Documents/Go%20Records/zcacrgw%202d%20-acn%202d%203oct2026.sgf', 'windows',
     'C:/Users/richard/Documents/Go Records/zcacrgw 2d -acn 2d 3oct2026.sgf'],
    ['file:///D:/SGF/%C3%A9tude%20100%25%20%231.sgf', 'windows', 'D:/SGF/étude 100% #1.sgf'],
    ['file:///C:/literal%2520name.sgf', 'windows', 'C:/literal%20name.sgf'],
    ['file://server/Go%20Records/game.sgf', 'windows', '//server/Go Records/game.sgf'],
    ['file://localhost/C:/games/game.sgf', 'windows', 'C:/games/game.sgf'],
    ['file:///home/gerry/Go%20Records/game.sgf', 'linux', '/home/gerry/Go Records/game.sgf'],
    ['file:///Users/player/%E6%A3%8B.sgf', 'osx', '/Users/player/棋.sgf'],
    ['file:///C:/literal-posix-name.sgf', 'linux', '/C:/literal-posix-name.sgf'],
    ['', 'windows', ''],
    ['https://example.org/game.sgf', 'windows', ''],
];
for (const [url, os, expected] of cases)
    assert.equal(context.localPathFromUrl(url, os), expected, `${os}: ${url}`);
for (const [url, os, expected] of [
    ['file:///C:/', 'windows', 'C:/'],
    ['file:///', 'linux', '/'],
    ['file:///C:/Go%20Records/', 'windows', 'C:/Go Records'],
    ['file://server/share/', 'windows', '//server/share'],
]) assert.equal(context.directoryPathFromUrl(url, os), expected);
// The regression was bypassing the helper, so guard the actual dialog wiring too.
const main = fs.readFileSync(path.join(repo, 'bermuda-qt/src/qml/Main.qml'), 'utf8');
const database = fs.readFileSync(path.join(repo, 'bermuda-qt/src/qml/DatabaseImportDialog.qml'), 'utf8');
for (const id of ['openSgfDialog', 'saveSgfDialog']) {
    const start = main.indexOf(`id: ${id}`);
    const handler = main.slice(main.indexOf('onAccepted:', start)).split('\n    }')[0];
    assert.match(handler, /root\.localPathFromUrl\(selectedFile\)/, id);
}
assert.match(database, /FilePaths\.directoryPathFromUrl\(url, Qt\.platform\.os\)/);
assert.doesNotMatch(main + database, /decodeURIComponent\(/);
console.log(`File URL regression checks passed (${cases.length + 4} cases and dialog wiring).`);
