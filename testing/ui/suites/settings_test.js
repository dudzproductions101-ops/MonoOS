// settings_test.js – Settings page navigation and toggle logic tests.
'use strict';

function assert(cond, msg)      { if (!cond) throw new Error(msg || 'assertion failed'); }
function assertEqual(a, b, msg) { if (a !== b) throw new Error(msg || `${a} !== ${b}`); }

class SettingsState {
    constructor() {
        this.route     = 'main';
        this.stack     = ['main'];
        this.toggles   = {
            wifi:         true,
            bluetooth:    false,
            darkMode:     true,
            notifications: true,
            locationServices: true,
            adb:          false,
        };
        this.theme     = 'dark';   // 'light' | 'dark' | 'amoled'
        this.textScale = 1.0;
    }

    navigate(route) {
        this.stack.push(route);
        this.route = route;
    }

    goBack() {
        if (this.stack.length > 1) {
            this.stack.pop();
            this.route = this.stack[this.stack.length - 1];
        }
    }

    toggle(id) {
        if (id in this.toggles)
            this.toggles[id] = !this.toggles[id];
    }

    setTheme(name) {
        if (['light', 'dark', 'amoled'].includes(name))
            this.theme = name;
    }

    setTextScale(v) {
        this.textScale = Math.max(0.8, Math.min(1.4, v));
    }
}

module.exports.tests = {
    'starts on main route': () => {
        const s = new SettingsState();
        assertEqual(s.route, 'main');
    },

    'navigate pushes to stack': () => {
        const s = new SettingsState();
        s.navigate('privacy');
        assertEqual(s.route, 'privacy');
        assertEqual(s.stack.length, 2);
    },

    'goBack pops stack': () => {
        const s = new SettingsState();
        s.navigate('security');
        s.goBack();
        assertEqual(s.route, 'main');
        assertEqual(s.stack.length, 1);
    },

    'goBack at root is safe': () => {
        const s = new SettingsState();
        s.goBack();
        assertEqual(s.route, 'main');
    },

    'toggle flips boolean': () => {
        const s = new SettingsState();
        assert(s.toggles.wifi);
        s.toggle('wifi');
        assert(!s.toggles.wifi);
        s.toggle('wifi');
        assert(s.toggles.wifi);
    },

    'toggle unknown id is safe': () => {
        const s = new SettingsState();
        s.toggle('nonexistent'); // must not throw
        assert(true);
    },

    'setTheme accepts valid values': () => {
        const s = new SettingsState();
        s.setTheme('light');
        assertEqual(s.theme, 'light');
        s.setTheme('amoled');
        assertEqual(s.theme, 'amoled');
    },

    'setTheme rejects invalid value': () => {
        const s = new SettingsState();
        s.setTheme('rainbow');
        assertEqual(s.theme, 'dark'); // unchanged
    },

    'textScale clamps to [0.8, 1.4]': () => {
        const s = new SettingsState();
        s.setTextScale(0.5);
        assertEqual(s.textScale, 0.8);
        s.setTextScale(2.0);
        assertEqual(s.textScale, 1.4);
        s.setTextScale(1.1);
        assertEqual(s.textScale, 1.1);
    },
};
