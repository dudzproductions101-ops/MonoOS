// notifications_test.js – Notification shade and card logic tests.
'use strict';

function assert(cond, msg) { if (!cond) throw new Error(msg || 'failed'); }
function assertEqual(a, b, m) { if (a !== b) throw new Error(m || `${a} !== ${b}`); }

class NotifState {
    constructor() {
        this.open          = false;
        this.notifications = [];
        this.nextId        = 1;
    }

    post({ title, body, packageName, priority = 2 }) {
        this.notifications.push({ id: this.nextId++, title, body, packageName, priority, dismissed: false });
    }

    dismiss(id) {
        const n = this.notifications.find(n => n.id === id);
        if (n) n.dismissed = true;
    }

    clearAll() { this.notifications.forEach(n => n.dismissed = true); }

    get active() { return this.notifications.filter(n => !n.dismissed); }

    get hasNotifications() { return this.active.length > 0; }

    ranked() {
        return [...this.active].sort((a, b) => b.priority - a.priority);
    }
}

module.exports.tests = {
    'starts empty and closed': () => {
        const s = new NotifState();
        assert(!s.open);
        assertEqual(s.active.length, 0);
    },

    'posting adds a notification': () => {
        const s = new NotifState();
        s.post({ title: 'Hello', body: 'World', packageName: 'com.test' });
        assertEqual(s.active.length, 1);
        assert(s.hasNotifications);
    },

    'dismiss removes from active': () => {
        const s = new NotifState();
        s.post({ title: 'A', body: 'B', packageName: 'com.a' });
        const id = s.notifications[0].id;
        s.dismiss(id);
        assertEqual(s.active.length, 0);
        assert(!s.hasNotifications);
    },

    'clearAll removes all': () => {
        const s = new NotifState();
        s.post({ title: '1', body: '', packageName: 'com.a' });
        s.post({ title: '2', body: '', packageName: 'com.b' });
        s.clearAll();
        assertEqual(s.active.length, 0);
    },

    'ranked sorts by priority descending': () => {
        const s = new NotifState();
        s.post({ title: 'low',  body: '', packageName: 'com.a', priority: 1 });
        s.post({ title: 'high', body: '', packageName: 'com.b', priority: 4 });
        s.post({ title: 'mid',  body: '', packageName: 'com.c', priority: 2 });
        const r = s.ranked();
        assertEqual(r[0].title, 'high');
        assertEqual(r[2].title, 'low');
    },

    'dismiss non-existent id is safe': () => {
        const s = new NotifState();
        s.dismiss(9999); // must not throw
        assert(true);
    },

    'multiple packages isolated': () => {
        const s = new NotifState();
        s.post({ title: 'A', body: '', packageName: 'com.a' });
        s.post({ title: 'B', body: '', packageName: 'com.b' });
        assertEqual(s.active.length, 2);
        s.dismiss(s.notifications[0].id);
        assertEqual(s.active.length, 1);
        assertEqual(s.active[0].packageName, 'com.b');
    },
};
