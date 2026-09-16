// Home dashboards, one per role. Reads computed data from the mock and renders.
import { html, render } from '../core/html.js';
import { delegate } from '../core/dom.js';
import { t } from '../core/i18n.js';
import { formatRupees, timeAgo } from '../core/format.js';
import { go } from '../core/router.js';
import * as commands from '../api/commands.js';
import { statCard, emptyState, activityRow } from '../components/widgets.js';
import { openCollectPicker } from './fees/collect.js';
import { openDayBook } from './fees/daybook.js';

function hero(name, sub, buttons) {
  const now = new Date();
  const hour = now.getHours();
  const greetKey = hour < 12 ? 'home.morning' : hour < 17 ? 'home.afternoon' : 'home.evening';
  const date = now.toLocaleDateString('en-IN', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
  return html`<div
    class="card"
    style="background:linear-gradient(120deg,var(--blue),var(--blue-d));border:none;color:#fff"
  >
    <div class="cb spread wrap g16">
      <div>
        <div style="font-size:13px;opacity:0.85">${date}</div>
        <div style="font-size:23px;font-weight:750;letter-spacing:-0.5px;margin-top:2px">
          ${t(greetKey)}, ${name.split(' ')[0]}
        </div>
        <div style="font-size:13.5px;opacity:0.9;margin-top:4px">${sub}</div>
      </div>
      <div class="row g10 wrap">${buttons}</div>
    </div>
  </div>`;
}
const heroBtn = (label, action, solid) =>
  html`<button
    class="btn"
    data-action="${action}"
    style="${solid ? 'background:#fff;color:var(--blue-d)' : 'background:rgba(255,255,255,.16);color:#fff;border:1px solid rgba(255,255,255,.3)'}"
  >
    ${label}
  </button>`;

export async function view(root, params) {
  const role = params.me.role;
  if (role === 'principal') return renderPrincipal(root, params);
  if (role === 'accountant') return renderAccountant(root);
  return renderTeacher(root);
}

async function renderPrincipal(root, params) {
  const d = await commands.homePrincipal();
  if (params.refreshChrome) {
    params.refreshChrome(
      html`<button
        class="pill ${d.backupOverdue ? 'p-orange' : 'p-green'}"
        style="min-height:38px"
        data-action="backup"
      >
        💾
        ${d.lastBackupAt ? t('home.backupAgo', { ago: timeAgo(d.lastBackupAt, Date.now()) }) : t('home.noBackup')}
      </button>`,
    );
  }
  const now = Date.now();
  render(
    root,
    html`<div class="inner stack">
      ${hero(
        d.name,
        t('home.principalSub', {
          marked: d.sections - d.pending.length,
          total: d.sections,
          unpaid: d.fees.unpaid,
        }),
        html`${heroBtn('📋 ' + t('nav.attendance'), 'attendance')}${heroBtn('₹ ' + t('common.collectFee'), 'collect', true)}`,
      )}
      <div class="stats">
        ${statCard(t('home.students'), d.students, t('home.classesSections', { classes: d.classes, sections: d.sections }))}
        ${statCard(
          t('home.presentToday'),
          d.attendance.done ? d.attendance.p : '—',
          d.attendance.done
            ? t('home.absentLeave', { absent: d.attendance.a, leave: d.attendance.l })
            : t('home.noSectionMarked'),
          'var(--green)',
        )}
        ${statCard(t('home.feesCollected'), formatRupees(d.fees.paid), t('home.ofTotal', { pct: d.fees.due ? Math.round((d.fees.paid / d.fees.due) * 100) : 0, total: formatRupees(d.fees.due) }), 'var(--blue)')}
        ${statCard(t('home.feesPending'), formatRupees(d.fees.pending), t('home.unpaidPart', { unpaid: d.fees.unpaid, part: d.fees.part }), 'var(--orange)')}
      </div>
      <div class="card">
        <div class="ch">
          <div>
            <h3>${t('home.attendancePending')}</h3>
            <p>
              ${d.pending.length ? t('home.sectionsWaiting', { n: d.pending.length }) : t('home.allDone')}
            </p>
          </div>
        </div>
        ${
          d.pending.length
            ? html`${d.pending.slice(0, 8).map(
                (p) =>
                  html`<div class="feerow">
                    <span class="pill p-blue" style="min-width:58px;justify-content:center">${p.ck}</span>
                    <div class="grow">
                      <b class="sm">${t('home.nStudents', { n: p.count })}</b>
                      <div class="xs mut">${p.teachers.join(', ') || t('home.noTeacher')}</div>
                    </div>
                    <button class="btn b-pri b-sm" data-action="mark" data-ck="${p.ck}">
                      ${t('home.mark')}
                    </button>
                  </div>`,
              )}`
            : emptyState('✅', t('home.everyMarked'))
        }
      </div>
      <div class="card">
        <div class="ch">
          <h3>${t('home.recentActivity')}</h3>
          <button class="btn b-quiet b-sm" data-action="activity">${t('home.seeAll')}</button>
        </div>
        ${d.recentActivity.length ? d.recentActivity.map((c) => activityRow(c, timeAgo(c.at, now))) : emptyState('🕘', t('home.nothingYet'))}
      </div>
    </div>`,
  );
  delegate(root, {
    attendance: () => go('attendance'),
    collect: () => openCollectPicker(),
    backup: () => go('backup'),
    activity: () => go('activity'),
    mark: (e, el) => go('attendance', { ck: el.getAttribute('data-ck') }),
  });
}

async function renderAccountant(root) {
  const d = await commands.homeAccountant();
  render(
    root,
    html`<div class="inner stack">
      ${hero(
        d.name,
        t('home.accountantSub', { receipts: d.receiptsToday, pending: formatRupees(d.fees.pending) }),
        html`${heroBtn('₹ ' + t('common.collectFee'), 'collect', true)}`,
      )}
      <div class="stats">
        ${statCard(t('home.collectedToday'), formatRupees(d.collectedToday), t('home.byMode', { cash: formatRupees(d.byMode.Cash), upi: formatRupees(d.byMode.UPI), cheque: formatRupees(d.byMode.Cheque) }), 'var(--green)')}
        ${statCard(t('home.receiptsToday'), d.receiptsToday, t('home.numberedPrefix', { prefix: d.devicePrefix }))}
        ${statCard(t('home.pendingSession'), formatRupees(d.fees.pending), t('home.unpaidPart', { unpaid: d.fees.unpaid, part: d.fees.part }), 'var(--orange)')}
        ${statCard(t('home.collectedSession'), formatRupees(d.fees.paid), t('home.ofTotal', { pct: d.fees.due ? Math.round((d.fees.paid / d.fees.due) * 100) : 0, total: formatRupees(d.fees.due) }), 'var(--blue)')}
      </div>
      <div class="card">
        <div class="ch"><h3>${t('home.highestDues')}</h3></div>
        ${
          d.topDue.length
            ? d.topDue.map(
                (s) =>
                  html`<div class="feerow">
                    <div class="grow">
                      <b class="sm">${s.name}</b>
                      <div class="xs mut">${s.ck} | ${s.father}</div>
                    </div>
                    <b class="num" style="color:var(--red)">${formatRupees(s.balance)}</b>
                    <button class="btn b-pri b-sm" data-action="collect-one" data-adm="${s.adm}">
                      ${t('home.collect')}
                    </button>
                  </div>`,
              )
            : emptyState('✅', t('home.noDues'))
        }
      </div>
    </div>`,
  );
  delegate(root, {
    collect: () => openCollectPicker(),
    'collect-one': (e, el) => openCollectPicker(el.getAttribute('data-adm')),
    daybook: () => openDayBook(),
  });
}

async function renderTeacher(root) {
  const d = await commands.homeTeacher();
  if (!d.classes.length) {
    render(
      root,
      html`<div class="inner stack">
        ${hero(d.name, t('home.noClasses'), '')}
        <div class="card">${emptyState('🏫', t('home.askPrincipalClasses'), t('home.onceAssigned'))}</div>
      </div>`,
    );
    return;
  }
  render(
    root,
    html`<div class="inner stack">
      ${hero(
        d.name,
        d.pending.length
          ? t('home.attendanceStillFor', { classes: d.pending.join(', ') })
          : t('home.attendanceDone'),
        html`${heroBtn('📋 ' + t('home.takeAttendance'), 'attendance', true)}${heroBtn('📝 ' + t('home.enterMarks'), 'marks')}`,
      )}
      <div class="sect">${t('home.yourClasses')}</div>
      <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:12px">
        ${d.classes.map(
          (c) =>
            html`<div class="card">
              <div class="ch">
                <div>
                  <h3>${t('home.class', { ck: c.ck })}</h3>
                  <p>${t('home.nStudents', { n: c.count })}</p>
                </div>
                ${c.marked ? html`<span class="pill p-green">✓ ${t('home.nPresent', { n: c.present })}</span>` : html`<span class="pill p-orange">${t('home.notMarked')}</span>`}
              </div>
              <div class="cb">
                <div class="row g8" style="margin-top:6px">
                  <button
                    class="btn ${c.marked ? 'b-out' : 'b-pri'} b-sm grow"
                    data-action="att"
                    data-ck="${c.ck}"
                  >
                    ${c.marked ? t('home.viewAttendance') : t('home.takeAttendance')}
                  </button>
                  <button class="btn b-out b-sm grow" data-action="mk" data-ck="${c.ck}">
                    ${t('nav.marks')}
                  </button>
                </div>
              </div>
            </div>`,
        )}
      </div>
    </div>`,
  );
  delegate(root, {
    attendance: () => go('attendance'),
    marks: () => go('marks'),
    att: (e, el) => go('attendance', { ck: el.getAttribute('data-ck') }),
    mk: (e, el) => go('marks', { ck: el.getAttribute('data-ck') }),
  });
}
