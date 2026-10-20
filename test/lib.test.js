import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildCommand, filterSessions, shortenHome } from '../src/lib.js';

test('buildCommand', () => {
  assert.equal(buildCommand({ host: 'h', user: 'u', port: 22, keyPath: '' }), 'ssh u@h');
  assert.equal(buildCommand({ host: 'h', user: 'u', port: 2222, keyPath: '~/.ssh/k' }), 'ssh -i ~/.ssh/k -p 2222 u@h');
  assert.equal(buildCommand({ host: 'h', user: '', port: '22', keyPath: '/My Keys/k' }), 'ssh -i "/My Keys/k" h');
});

test('shortenHome rewrites paths under home to ~/, leaves others', () => {
  assert.equal(shortenHome('/Users/me/.ssh/k', '/Users/me'), '~/.ssh/k');
  assert.equal(shortenHome('C:\\Users\\me\\.ssh\\k', 'C:\\Users\\me'), '~/.ssh/k');
  assert.equal(shortenHome('/etc/ssh/k', '/Users/me'), '/etc/ssh/k');
});

test('filterSessions matches name, host and user, case-insensitive', () => {
  const s = [
    { name: 'prod', host: '10.0.0.1', user: 'root' },
    { name: 'dev', host: 'dev.local', user: 'me' },
  ];
  assert.deepEqual(filterSessions(s, 'ROOT').map((x) => x.name), ['prod']);
  assert.deepEqual(filterSessions(s, '.local').map((x) => x.name), ['dev']);
  assert.equal(filterSessions(s, '').length, 2);
});
