import { buildCommand, filterSessions } from './lib.js';

const { invoke } = window.__TAURI__.core;
const IS_WIN = navigator.userAgent.includes('Windows');
const MOD = IS_WIN ? 'Ctrl' : '⌘';
const TERMINALS = IS_WIN
  ? [['wt', 'Windows Terminal'], ['powershell', 'PowerShell'], ['cmd', 'Command Prompt']]
  : [['terminal', 'Terminal'], ['iterm2', 'iTerm2'], ['warp', 'Warp'], ['alacritty', 'Alacritty'], ['ghostty', 'Ghostty']];

let sessions = [];
let filtered = [];
let selectedIndex = 0;
let editing = null; // draft being edited; id === null means new
let confirmDelete = null; // timer for two-step delete
let toastTimer = null;

const $ = (id) => document.getElementById(id);
const searchInput = $('searchInput');
const sessionList = $('sessionList');
const detailView = $('detailView');

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
const termLabel = (id) => (TERMINALS.find(([k]) => k === id) || [id, id])[1];
const termId = (id) => (TERMINALS.some(([k]) => k === id) ? id : TERMINALS[0][0]);
const current = () => filtered[selectedIndex];

const FIELD = 'w-full bg-krom-black text-sm text-white px-3 py-2 rounded border border-krom-border focus:border-krom-cyan';
const LABEL = 'text-[10px] tracking-widest text-krom-muted uppercase mb-1 block';
const BTN = 'px-4 py-2.5 rounded border border-krom-border text-sm text-neutral-300 hover:text-white hover:border-neutral-600 transition';
const PRIMARY = 'flex-1 bg-krom-red hover:bg-[#ff4f64] text-white font-medium py-2.5 rounded transition shadow-[0_0_15px_rgba(237,64,84,0.3)]';

async function load() {
  try {
    sessions = JSON.parse(await invoke('load_sessions'));
  } catch (e) {
    sessions = [];
    toast(`Could not load sessions: ${e}`);
  }
  applyFilter();
}

async function persist() {
  try {
    await invoke('save_sessions', { data: JSON.stringify(sessions, null, 2) });
  } catch (e) {
    toast(`Save failed: ${e}`);
  }
}

function applyFilter() {
  filtered = filterSessions(sessions, searchInput.value);
  selectedIndex = Math.min(selectedIndex, Math.max(0, filtered.length - 1));
  render();
}

function render() {
  renderList();
  if (!editing) renderDetails();
}

function renderList() {
  if (filtered.length === 0) {
    sessionList.innerHTML = `<div class="px-6 py-4 text-sm text-krom-muted">${sessions.length ? 'No sessions found.' : 'No sessions yet.'}</div>`;
    return;
  }
  sessionList.innerHTML = filtered
    .map((s, i) => {
      const cls = i === selectedIndex
        ? 'border-krom-red bg-krom-gray text-white'
        : 'border-transparent text-neutral-400 hover:text-white hover:bg-neutral-900';
      return `<div data-index="${i}" class="px-6 py-3 cursor-pointer border-l-2 flex flex-col gap-1 transition-all ${cls}">
        <div class="font-medium text-sm">${esc(s.name)}</div>
        <div class="font-mono text-[10px] opacity-60">${esc(s.user ? `${s.user}@${s.host}` : s.host)}</div>
      </div>`;
    })
    .join('');
  sessionList.querySelector(`[data-index="${selectedIndex}"]`)?.scrollIntoView({ block: 'nearest' });
}

function renderDetails() {
  const s = current();
  if (!s) {
    detailView.innerHTML = `<div class="my-auto text-center text-sm text-krom-muted">${
      sessions.length
        ? 'Nothing matches.'
        : `Press <span class="text-krom-cyan">${MOD}N</span> or <span class="text-krom-cyan">+ New</span> to add your first session.`
    }</div>`;
    return;
  }
  const options = TERMINALS.map(([id, label]) => `<option value="${id}" ${termId(s.terminal) === id ? 'selected' : ''}>${label}</option>`).join('');
  detailView.innerHTML = `
    <div class="my-auto animate-fadeIn space-y-8">
      <div>
        <h2 class="text-2xl font-semibold mb-2">${esc(s.name)}</h2>
        <div class="font-mono text-sm text-krom-cyan">${esc(s.user ? `${s.user}@` : '')}${esc(s.host)}:${esc(s.port || 22)}</div>
      </div>
      <div class="space-y-4">
        <div>
          <label class="${LABEL}">Key Path</label>
          <div class="font-mono text-xs text-neutral-300 bg-krom-black px-3 py-2 rounded border border-krom-border min-h-[34px]">${esc(s.keyPath) || '<span class="text-krom-muted">default key / agent</span>'}</div>
        </div>
        <div>
          <label class="${LABEL}">Terminal</label>
          <select id="termSelect" class="${FIELD} appearance-none cursor-pointer">${options}</select>
        </div>
        <div>
          <label class="${LABEL} flex justify-between">
            <span>Command</span>
            <button data-action="copy" class="text-krom-cyan hover:text-white transition">Copy</button>
          </label>
          <div class="font-mono text-xs text-krom-cyan bg-krom-black px-3 py-2 rounded border border-krom-border overflow-x-auto whitespace-nowrap">${esc(buildCommand(s))}</div>
        </div>
      </div>
      <div class="pt-4 flex gap-3">
        <button data-action="connect" class="${PRIMARY}">Connect Session</button>
        <button data-action="edit" class="${BTN}">Edit</button>
        <button data-action="delete" class="${BTN} hover:text-krom-red hover:border-krom-red">Delete</button>
      </div>
    </div>`;
}

function renderForm() {
  const d = editing;
  const input = (name, label, extra = '') =>
    `<div><label class="${LABEL}">${label}</label><input name="${name}" value="${esc(d[name])}" class="${FIELD} font-mono" ${extra} /></div>`;
  const options = TERMINALS.map(([id, label]) => `<option value="${id}" ${termId(d.terminal) === id ? 'selected' : ''}>${label}</option>`).join('');
  detailView.innerHTML = `
    <form id="sessionForm" class="my-auto animate-fadeIn space-y-4" autocomplete="off">
      <h2 class="text-2xl font-semibold mb-4">${d.id ? 'Edit session' : 'New session'}</h2>
      ${input('name', 'Name', 'required')}
      <div class="grid grid-cols-[1fr_1fr_90px] gap-3">
        ${input('host', 'Host', 'required')}
        ${input('user', 'User')}
        ${input('port', 'Port', 'type="number" min="1" max="65535"')}
      </div>
      ${input('keyPath', 'Key Path', 'placeholder="~/.ssh/id_ed25519"')}
      <div>
        <label class="${LABEL}">Terminal</label>
        <select name="terminal" class="${FIELD} appearance-none cursor-pointer">${options}</select>
      </div>
      <div class="pt-4 flex gap-3">
        <button type="submit" class="${PRIMARY}">Save</button>
        <button type="button" data-action="cancel" class="${BTN}">Cancel</button>
      </div>
    </form>`;
  detailView.querySelector('[name=name]').focus();
}

function startEdit(s) {
  editing = s ? { ...s } : { id: null, name: '', host: '', user: '', port: 22, keyPath: '', terminal: TERMINALS[0][0] };
  renderForm();
}

async function saveForm() {
  const d = { ...editing };
  for (const k of ['name', 'host', 'user', 'keyPath']) d[k] = String(d[k] ?? '').trim();
  d.port = Number(d.port) || 22;
  d.terminal = termId(d.terminal);
  if (!d.name || !d.host) return toast('Name and host are required');
  if (d.id) {
    sessions = sessions.map((s) => (s.id === d.id ? d : s));
  } else {
    d.id = crypto.randomUUID();
    sessions.push(d);
  }
  editing = null;
  searchInput.value = '';
  await persist();
  filtered = filterSessions(sessions, '');
  selectedIndex = Math.max(0, filtered.findIndex((s) => s.id === d.id));
  render();
  toast(`Saved ${d.name}`);
  searchInput.focus();
}

function cancelEdit() {
  editing = null;
  render();
  searchInput.focus();
}

async function deleteCurrent(btn) {
  const s = current();
  if (!s) return;
  if (!confirmDelete) {
    btn.textContent = 'Confirm?';
    confirmDelete = setTimeout(() => { confirmDelete = null; renderDetails(); }, 3000);
    return;
  }
  clearTimeout(confirmDelete);
  confirmDelete = null;
  sessions = sessions.filter((x) => x.id !== s.id);
  await persist();
  applyFilter();
  toast(`Deleted ${s.name}`);
}

async function connect(s) {
  if (!s) return;
  const terminal = termId(s.terminal);
  toast(`Opening ${s.name} in ${termLabel(terminal)}…`);
  try {
    await invoke('open_terminal', { terminal, title: s.name, command: buildCommand(s) });
  } catch (e) {
    toast(`Failed: ${e}`);
  }
}

async function copyCommand() {
  const s = current();
  if (!s) return;
  const cmd = buildCommand(s);
  try {
    await navigator.clipboard.writeText(cmd);
  } catch {
    const ta = document.createElement('textarea');
    ta.value = cmd;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    ta.remove();
  }
  toast('Command copied to clipboard');
}

function toast(msg) {
  const t = $('toast');
  $('toast-message').textContent = msg;
  t.classList.remove('translate-y-20', 'opacity-0');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.add('translate-y-20', 'opacity-0'), 2500);
}

searchInput.addEventListener('input', () => {
  selectedIndex = 0;
  applyFilter();
});

sessionList.addEventListener('click', (e) => {
  const el = e.target.closest('[data-index]');
  if (!el) return;
  selectedIndex = Number(el.dataset.index);
  editing = null;
  render();
});

detailView.addEventListener('click', (e) => {
  const btn = e.target.closest('[data-action]');
  if (!btn) return;
  const a = btn.dataset.action;
  if (a === 'connect') connect(current());
  else if (a === 'edit') startEdit(current());
  else if (a === 'delete') deleteCurrent(btn);
  else if (a === 'copy') copyCommand();
  else if (a === 'cancel') cancelEdit();
});

detailView.addEventListener('submit', (e) => {
  e.preventDefault();
  saveForm();
});

detailView.addEventListener('input', (e) => {
  if (editing && e.target.name) editing[e.target.name] = e.target.value;
});

detailView.addEventListener('change', (e) => {
  if (editing && e.target.name) editing[e.target.name] = e.target.value;
  if (editing || e.target.id !== 'termSelect') return;
  const s = current();
  if (!s) return;
  s.terminal = e.target.value;
  persist();
});

$('newBtn').addEventListener('click', () => startEdit(null));

document.addEventListener('keydown', (e) => {
  const mod = IS_WIN ? e.ctrlKey : e.metaKey;
  const key = e.key.toLowerCase();
  if (mod && key === 'k') { e.preventDefault(); searchInput.focus(); searchInput.select(); return; }
  if (mod && key === 'n') { e.preventDefault(); startEdit(null); return; }
  if (editing) {
    if (e.key === 'Escape') { e.preventDefault(); cancelEdit(); }
    return;
  }
  const tag = document.activeElement?.tagName;
  if (document.activeElement !== searchInput && e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey && tag !== 'SELECT') {
    searchInput.focus();
  }
  if (filtered.length === 0) return;
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    selectedIndex = (selectedIndex + 1) % filtered.length;
    render();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    selectedIndex = (selectedIndex - 1 + filtered.length) % filtered.length;
    render();
  } else if (e.key === 'Enter' && tag !== 'BUTTON') {
    e.preventDefault();
    connect(current());
  }
});

async function checkForUpdate() {
  try {
    const update = await window.__TAURI__.updater.check();
    if (!update) return;
    const bar = $('updateBar');
    const btn = $('updateBtn');
    $('updateMsg').textContent = `Version ${update.version} is available`;
    bar.classList.remove('hidden');
    $('updateLater').onclick = () => bar.classList.add('hidden');
    btn.onclick = async () => {
      btn.disabled = true;
      btn.textContent = 'Installing…';
      try {
        await update.downloadAndInstall();
        await window.__TAURI__.process.relaunch();
      } catch (e) {
        toast(`Update failed: ${e}`);
        bar.classList.add('hidden');
      }
    };
  } catch {
    // offline, dev build or no release yet: stay quiet
  }
}

$('kbdHint').textContent = `${MOD}K`;
load().then(() => searchInput.focus());
checkForUpdate();
