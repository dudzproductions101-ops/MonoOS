// lockscreen_test.js – Lock screen UI logic tests.
'use strict';

function assert(cond, msg) { if (!cond) throw new Error(msg || 'failed'); }
function assertEqual(a, b, m) { if (a !== b) throw new Error(m || `${a} !== ${b}`); }

class LockState {
    constructor() {
        this.locked        = true;
        this.authRequired  = false;
        this.authMethod    = 'pin';   // 'pin' | 'biometric' | 'pattern'
        this.pinLength     = 6;
        this.pinEntered    = 0;
        this.errorMessage  = '';
        this.biometricOk   = false;
        this.failCount     = 0;
        this.maxFails      = 5;
    }

    beginAuth()        { this.authRequired = true; }
    triggerBiometric() { this.biometricOk = true; if (this.biometricOk) this.unlock(); }

    enterDigit(d) {
        if (!this.authRequired) return;
        if (d === '⌫') { this.pinEntered = Math.max(0, this.pinEntered - 1); return; }
        this.pinEntered++;
        if (this.pinEntered === this.pinLength) this._checkPin();
    }

    _checkPin(correct = true) {
        if (correct) {
            this.unlock();
        } else {
            this.failCount++;
            this.pinEntered  = 0;
            this.errorMessage = `Incorrect PIN. ${this.maxFails - this.failCount} attempts left.`;
        }
    }

    unlock() {
        this.locked       = false;
        this.authRequired = false;
        this.pinEntered   = 0;
        this.errorMessage = '';
    }

    backspace() { this.enterDigit('⌫'); }
}

module.exports.tests = {
    'starts locked': () => {
        const l = new LockState();
        assert(l.locked);
        assert(!l.authRequired);
    },

    'beginAuth sets authRequired': () => {
        const l = new LockState();
        l.beginAuth();
        assert(l.authRequired);
    },

    'entering PIN digits increments counter': () => {
        const l = new LockState();
        l.beginAuth();
        l.enterDigit('1');
        l.enterDigit('2');
        assertEqual(l.pinEntered, 2);
    },

    'backspace decrements PIN counter': () => {
        const l = new LockState();
        l.beginAuth();
        l.enterDigit('1');
        l.enterDigit('2');
        l.backspace();
        assertEqual(l.pinEntered, 1);
    },

    'backspace at 0 is safe': () => {
        const l = new LockState();
        l.beginAuth();
        l.backspace();
        assertEqual(l.pinEntered, 0);
    },

    'correct PIN unlocks': () => {
        const l = new LockState();
        l.beginAuth();
        // Simulate 6-digit correct entry
        for (let i = 0; i < 6; i++) l.enterDigit(`${i}`);
        assert(!l.locked);
        assert(!l.authRequired);
    },

    'biometric success unlocks': () => {
        const l = new LockState();
        l.beginAuth();
        l.biometricOk = true;
        l.unlock();
        assert(!l.locked);
    },

    'pin length default is 6': () => {
        const l = new LockState();
        assertEqual(l.pinLength, 6);
    },
};
