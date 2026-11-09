'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const installer = require('../bin/ai-switcher.cjs');

function fixture(t) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'aisw-installer-test-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}

test('checksum verifies filenames with spaces and rejects tampered or ambiguous packages', (t) => {
  const root = fixture(t);
  const file = path.join(root, 'AI switcher_0.12.0_amd64.deb');
  fs.writeFileSync(file, 'fixture package');
  const hash = crypto.createHash('sha256').update('fixture package').digest('hex');
  assert.equal(installer.verifyChecksum(file, `${hash}  ${path.basename(file)}\n`), hash);
  assert.throws(() => installer.verifyChecksum(file, `${hash}  other.deb\n`));
  assert.throws(() => installer.verifyChecksum(file, `${hash}  ${path.basename(file)}\n${hash}  ${path.basename(file)}\n`));
  fs.appendFileSync(file, 'tampered');
  assert.throws(() => installer.verifyChecksum(file, `${hash}  ${path.basename(file)}\n`));
});

test('unsupported platforms fail before any installation', () => {
  assert.doesNotThrow(() => installer.assertPlatform('linux', 'x64'));
  assert.throws(() => installer.assertPlatform('linux', 'arm64'));
  assert.throws(() => installer.assertPlatform('darwin', 'x64'));
});

test('desktop command quotes spaces and prevents desktop field injection', () => {
  assert.equal(installer.desktopQuote('/home/a user/100%/bin'), '"/home/a user/100%%/bin"');
  assert.equal(installer.desktopQuote('/home/a"$`\\/bin'), '"/home/a\\"\\$\\`\\\\/bin"');
  assert.throws(() => installer.desktopQuote('/home/name\nExec=other'));
});

test('installer refuses unrelated directories and launchers', (t) => {
  const home = fixture(t);
  const paths = installer.locations(home, {});
  fs.mkdirSync(paths.root, { recursive: true });
  fs.writeFileSync(path.join(paths.root, 'important.txt'), 'untouched');
  assert.throws(() => installer.assertManagedPaths(paths));
  assert.equal(fs.readFileSync(path.join(paths.root, 'important.txt'), 'utf8'), 'untouched');
  fs.rmSync(paths.root, { recursive: true });
  fs.mkdirSync(path.dirname(paths.launcher), { recursive: true });
  fs.writeFileSync(paths.launcher, 'unrelated launcher');
  assert.throws(() => installer.assertManagedPaths(paths));
  assert.equal(fs.readFileSync(paths.launcher, 'utf8'), 'unrelated launcher');
});

test('existing extracted AI switcher and its launcher are accepted without touching them', (t) => {
  const home = fixture(t);
  const paths = installer.locations(home, {});
  fs.mkdirSync(path.dirname(paths.binary), { recursive: true });
  fs.writeFileSync(paths.binary, 'fixture binary');
  const desktop = path.join(paths.root, 'usr/share/applications/AI switcher.desktop');
  fs.mkdirSync(path.dirname(desktop), { recursive: true });
  fs.writeFileSync(desktop, '[Desktop Entry]\nName=AI switcher\n');
  fs.mkdirSync(path.dirname(paths.launcher), { recursive: true });
  fs.symlinkSync(paths.binary, paths.launcher);
  assert.doesNotThrow(() => installer.assertManagedPaths(paths));
});
