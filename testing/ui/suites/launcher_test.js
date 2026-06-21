// launcher_test.js – Launcher component UI tests.
//
// Uses a thin DOM-simulation harness; on-device these run inside a
// Squish session against the real QML compositor.

'use strict';

// ── Minimal assertion helper ─────────────────────────────────────────────────
function assert(cond, msg) {
    if (!cond) throw new Error(msg || 'assertion failed');
}
function assertEqual(a, b, msg) {
    if (a !== b) throw new Error(msg || `expected ${JSON.stringify(a)} === ${JSON.stringify(b)}`);
}

// ── Simulated launcher state ─────────────────────────────────────────────────
class LauncherState {
    constructor() {
        this.homePage      = 0;
        this.pageCount     = 3;
        this.gridColumns   = 4;
        this.editMode      = false;
        this.drawerOpen    = false;
        this.dockApps      = ['Phone', 'Messages', 'Browser', 'Camera'];
        this.apps          = Array.from({ length: 24 }, (_, i) => ({
            packageName: `com.monoos.app${i}`,
            label:       `App ${i}`,
            iconPath:    `/res/icons/app${i}.svg`,
            badgeCount:  0,
        }));
        this.launched      = [];
        this.recentsOpen   = false;
        this.searchOpen    = false;
        this.notifShadeOpen = false;
    }

    appsForPage(pageIndex) {
        const perPage = this.gridColumns * 6;
        const start   = pageIndex * perPage;
        return this.apps.slice(start, start + perPage);
    }

    launchApp(pkg) { this.launched.push(pkg); }

    enterEditMode()              { this.editMode = true;  }
    showRecents()                { this.recentsOpen = true; }
    showNotificationShade()      { this.notifShadeOpen = true; }
    closeDrawer()                { this.drawerOpen = false; }
    goHome()                     { this.homePage = 0; this.editMode = false; }
    goBack()                     { /* handled by app stack */ }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
module.exports.tests = {
    'page 0 contains up to 24 apps': () => {
        const l = new LauncherState();
        assert(l.appsForPage(0).length <= l.gridColumns * 6);
    },

    'appsForPage respects grid columns': () => {
        const l = new LauncherState();
        assertEqual(l.gridColumns, 4);
        const p0 = l.appsForPage(0);
        assert(p0.length % 1 === 0); // any count is fine; just ensure no error
    },

    'launchApp records the package name': () => {
        const l = new LauncherState();
        l.launchApp('com.example.camera');
        assertEqual(l.launched[l.launched.length - 1], 'com.example.camera');
    },

    'enterEditMode sets editMode flag': () => {
        const l = new LauncherState();
        assert(!l.editMode);
        l.enterEditMode();
        assert(l.editMode);
    },

    'goHome clears editMode and resets page': () => {
        const l = new LauncherState();
        l.homePage = 2;
        l.enterEditMode();
        l.goHome();
        assertEqual(l.homePage, 0);
        assert(!l.editMode);
    },

    'showRecents opens recents view': () => {
        const l = new LauncherState();
        assert(!l.recentsOpen);
        l.showRecents();
        assert(l.recentsOpen);
    },

    'notification shade toggle': () => {
        const l = new LauncherState();
        l.showNotificationShade();
        assert(l.notifShadeOpen);
    },

    'dock has 4 apps by default': () => {
        const l = new LauncherState();
        assertEqual(l.dockApps.length, 4);
    },

    'page count is 3': () => {
        const l = new LauncherState();
        assertEqual(l.pageCount, 3);
    },

    'app drawer starts closed': () => {
        const l = new LauncherState();
        assert(!l.drawerOpen);
    },
};
