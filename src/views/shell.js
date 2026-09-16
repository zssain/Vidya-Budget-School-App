// App shell and controller: builds the chrome (nav, top bar, tab bar), manages
// the setup/login/app overlays, navigation, language and the account modal.
import { canView, currentView, getView, go, navItems, onNavigate, rerender } from '../core/router.js';
import * as commands from '../api/commands.js';
import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { getLanguage, onLanguageChange, setLanguage, t } from '../core/i18n.js';
import { confirmDialog, openModal, toast } from '../core/ui.js';
import { toAppError } from '../api/errors.js';
import { renderWelcome } from './setup/welcome.js';
import { renderLogin } from './login.js';
import { renderPasswordModal } from './password.js';

let me = null; // current user DTO
let els = null;

function initials(name) {
  return String(name || '?')
    .trim()
    .split(/\s+/)
    .map((w) => w[0])
    .slice(0, 2)
    .join('')
    .toUpperCase();
}

function buildChrome() {
  if (els) return;
  const setup = document.createElement('div');
  setup.id = 'setup';
  setup.className = 'hide';

  const login = document.createElement('div');
  login.id = 'login';
  login.className = 'hide';

  const app = document.getElementById('app') || document.createElement('div');
  app.id = 'app';
  render(
    app,
    html`<nav class="nav">
        <div class="nav-top">
          <div class="logo">वि</div>
          <div><b>Vidya</b><span id="navSchool"></span></div>
        </div>
        <div class="nav-list" id="navList"></div>
        <div class="nav-foot">
          <button class="nav-user" data-action="account">
            <div class="av" id="userAv"></div>
            <div style="min-width:0"><b id="userName"></b><span id="userRole"></span></div>
          </button>
        </div>
      </nav>
      <div class="main">
        <header class="top">
          <h2 id="pageTitle"></h2>
          <div class="grow"></div>
          <span id="topPill"></span>
          <div class="lang">
            <button id="langEN" data-action="lang-en">EN</button
            ><button id="langHI" data-action="lang-hi">हिं</button>
          </div>
        </header>
        <main class="view" id="view"></main>
      </div>`,
  );

  const tabbar = document.createElement('div');
  tabbar.className = 'tabbar';
  tabbar.id = 'tabbar';

  document.body.append(setup, login, tabbar);
  if (!app.parentNode) document.body.insertBefore(app, tabbar);

  els = {
    setup,
    login,
    app,
    tabbar,
    view: app.querySelector('#view'),
    navList: app.querySelector('#navList'),
    pageTitle: app.querySelector('#pageTitle'),
    topPill: app.querySelector('#topPill'),
    userName: app.querySelector('#userName'),
    userRole: app.querySelector('#userRole'),
    userAv: app.querySelector('#userAv'),
    navSchool: app.querySelector('#navSchool'),
  };

  // Chrome-level actions (nav, tabs, account, language).
  delegate(app, {
    account: () => openAccount(),
    'lang-en': () => switchLang('en'),
    'lang-hi': () => switchLang('hi'),
    nav: (e, el) => go(el.getAttribute('data-view')),
  });
  delegate(tabbar, {
    nav: (e, el) => go(el.getAttribute('data-view')),
    more: () => openMore(),
  });

  onNavigate((cur) => mountView(cur));
  onLanguageChange(() => {
    if (me) {
      renderNav();
      refreshChrome();
      rerender();
    }
  });
}

function switchLang(lang) {
  setLanguage(lang);
  if (me) commands.setLanguage({ language: lang }).catch(() => {});
}

function show(which) {
  els.setup.classList.toggle('hide', which !== 'setup');
  els.login.classList.toggle('hide', which !== 'login');
  els.app.classList.toggle('on', which === 'app');
  if (which !== 'app') els.tabbar.replaceChildren();
}

export async function start() {
  buildChrome();
  updateLangButtons();
  try {
    const status = await commands.appStatus();
    if (!status.hasSchool) return showSetup();
    return showLogin();
  } catch {
    return showSetup();
  }
}

function ctx() {
  return { enterApp, showLogin, showSetup, showPasswordFor };
}

function showSetup() {
  me = null;
  show('setup');
  renderWelcome(els.setup, ctx());
}

export function showLogin(message) {
  me = null;
  show('login');
  renderLogin(els.login, ctx(), message);
}

function showPasswordFor(pendingToken) {
  renderPasswordModal({ forced: true, pendingToken, ctx: ctx() });
}

async function enterApp(user) {
  me = user;
  try {
    const st = await commands.appStatus();
    me.schoolName = st.schoolName;
  } catch {
    // ignore — the nav school label just stays empty
  }
  setLanguage(user.language || 'en');
  updateLangButtons();
  show('app');
  renderNav();
  refreshChrome();
  await go('home');
}

function updateLangButtons() {
  const lang = getLanguage();
  const en = els.app.querySelector('#langEN');
  const hi = els.app.querySelector('#langHI');
  if (en) en.className = lang === 'en' ? 'on' : '';
  if (hi) hi.className = lang === 'hi' ? 'on' : '';
}

function refreshChrome() {
  if (!me) return;
  els.userName.textContent = me.name;
  els.userRole.textContent = t('roles.' + me.role);
  els.userAv.textContent = initials(me.name);
  els.navSchool.textContent = me.schoolName || '';
  updateLangButtons();
  // Top pill is refreshed by the home view data; keep it simple here.
  render(els.topPill, html``);
}

function renderNav() {
  const items = navItems(me.permissions);
  render(
    els.navList,
    html`${items.map(
      (v) =>
        html`<button
          class="nav-item ${currentView().id === v.id ? 'on' : ''}"
          data-action="nav"
          data-view="${v.id}"
        >
          <span class="ico" style="background:${v.navBg || 'var(--line2)'}" aria-hidden="true"
            >${v.navIcon || ''}</span
          >
          <span>${t(v.title)}</span>
        </button>`,
    )}`,
  );
  const tabs = items.length > 5 ? items.slice(0, 4) : items;
  render(
    els.tabbar,
    html`${tabs.map(
      (v) =>
        html`<button class="${currentView().id === v.id ? 'on' : ''}" data-action="nav" data-view="${v.id}">
          <span class="ti">${v.navIcon || ''}</span><span>${t(v.title)}</span>
        </button>`,
    )}${
      items.length > 5
        ? html`<button data-action="more"><span class="ti">☰</span><span>${t('nav.more')}</span></button>`
        : ''
    }`,
  );
}

function openMore() {
  const items = navItems(me.permissions).slice(4);
  openModal({
    title: t('nav.more'),
    body: html`<div style="display:flex;flex-direction:column;gap:8px">
      ${items.map(
        (v) =>
          html`<button
            class="btn b-out"
            style="justify-content:flex-start"
            data-action="go"
            data-view="${v.id}"
          >
            ${v.navIcon || ''} ${t(v.title)}
          </button>`,
      )}
      <button class="btn b-out" style="justify-content:flex-start" data-action="account">
        👤 ${t('common.myAccount')}
      </button>
    </div>`,
    onAction: (name, { element, close }) => {
      if (name === 'go') {
        close();
        go(element.getAttribute('data-view'));
      } else if (name === 'account') {
        close();
        openAccount();
      }
    },
  });
}

function openAccount() {
  openModal({
    title: me.name,
    subtitle: t('roles.' + me.role),
    body: html`<div class="pl">
        <span class="k">${t('account.username')}</span><span class="v mono">${me.username}</span>
      </div>
      ${
        me.role === 'teacher'
          ? html`<div class="pl">
              <span class="k">${t('account.myClasses')}</span
              ><span class="v">${(me.sections || []).join(', ') || '—'}</span>
            </div>`
          : ''
      }`,
    footer: html`<button class="btn b-out" data-action="change-password">
        ${t('account.changePassword')}
      </button>
      <button class="btn b-pri" data-action="sign-out">${t('common.signOut')}</button>`,
    onAction: async (name, { close }) => {
      if (name === 'change-password') {
        close();
        renderPasswordModal({ forced: false });
      } else if (name === 'sign-out') {
        close();
        await commands.signOut();
        showLogin();
      }
    },
  });
}

async function mountView(cur) {
  if (!me) return;
  if (!cur.id || !canView(cur.id, me.permissions)) {
    if (cur.id) toast(t('errors.noAccess'));
    return go('home');
  }
  const view = getView(cur.id);
  renderNav();
  els.pageTitle.textContent = t(view.title);
  els.view.scrollTop = 0;
  try {
    await view.render(els.view, { ...(cur.params || {}), me, refreshChrome: applyTopPill });
  } catch (e) {
    const err = toAppError(e);
    render(els.view, html`<div class="inner"><div class="note n-red">${err.message}</div></div>`);
  }
}

// Views may set the top pill (e.g. principal backup status) via this callback.
function applyTopPill(safe) {
  if (safe) render(els.topPill, safe);
}

/** Expose the current user (used by tests/other modules if needed). */
export function currentMe() {
  return me;
}

export { confirmDialog };
