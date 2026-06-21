#!/usr/bin/env node
// run_tests.js – MonoOS UI test runner.
//
// Discovers and runs all *_test.js files under testing/ui/suites/,
// then aggregates results and exits non-zero on any failure.
//
// Usage:
//   node run_tests.js [--suite <name>] [--ci] [--reporter junit|dot]

'use strict';

const fs   = require('fs');
const path = require('path');

// ── Argument parsing ────────────────────────────────────────────────────────
const args    = process.argv.slice(2);
const ci      = args.includes('--ci');
const suiteArg = (() => {
    const i = args.indexOf('--suite');
    return i >= 0 ? args[i + 1] : null;
})();
const reporter = (() => {
    const i = args.indexOf('--reporter');
    return i >= 0 ? args[i + 1] : 'dot';
})();

// ── Discover test files ─────────────────────────────────────────────────────
const SUITES_DIR = path.join(__dirname, 'suites');

function discoverTests(dir, suiteName) {
    if (!fs.existsSync(dir)) return [];
    return fs.readdirSync(dir, { withFileTypes: true })
        .filter(e => e.isFile() && e.name.endsWith('_test.js'))
        .filter(e => !suiteName || e.name.startsWith(suiteName))
        .map(e => path.join(dir, e.name));
}

const testFiles = discoverTests(SUITES_DIR, suiteArg);

if (testFiles.length === 0) {
    console.error(`No test files found in ${SUITES_DIR}` +
                  (suiteArg ? ` matching '${suiteArg}'` : ''));
    process.exit(1);
}

// ── Minimal test harness ─────────────────────────────────────────────────────
let passed = 0, failed = 0, skipped = 0;
const failures = [];

async function runSuite(file) {
    const suiteName = path.basename(file, '.js');
    let suite;
    try {
        suite = require(file);
    } catch (e) {
        console.error(`  [ERROR] Failed to load ${file}: ${e.message}`);
        failed++;
        failures.push({ suite: suiteName, test: 'load', error: e.message });
        return;
    }

    if (typeof suite.tests !== 'object') return;

    for (const [name, fn] of Object.entries(suite.tests)) {
        try {
            await fn();
            passed++;
            if (reporter === 'dot') process.stdout.write('.');
            else console.log(`  [PASS] ${suiteName} :: ${name}`);
        } catch (e) {
            failed++;
            failures.push({ suite: suiteName, test: name, error: e.message });
            if (reporter === 'dot') process.stdout.write('F');
            else console.log(`  [FAIL] ${suiteName} :: ${name}: ${e.message}`);
        }
    }
}

// ── Main ────────────────────────────────────────────────────────────────────
(async () => {
    console.log(`\nMonoOS UI Tests  (${testFiles.length} suites)\n`);
    for (const f of testFiles) await runSuite(f);
    if (reporter === 'dot') console.log();
    console.log(`\n${'─'.repeat(50)}`);
    console.log(`Passed: ${passed}  Failed: ${failed}  Skipped: ${skipped}`);

    if (failures.length > 0) {
        console.log('\nFailures:');
        for (const { suite, test, error } of failures)
            console.log(`  ${suite} :: ${test}\n    ${error}`);
    }

    if (ci && reporter === 'junit') {
        // Emit JUnit XML for CI systems.
        const xml = [
            '<?xml version="1.0"?>',
            `<testsuite name="monoos-ui" tests="${passed + failed}" failures="${failed}">`,
            ...failures.map(({ suite, test, error }) =>
                `  <testcase classname="${suite}" name="${test}">` +
                `<failure>${error.replace(/</g,'&lt;')}</failure></testcase>`),
            '</testsuite>',
        ].join('\n');
        fs.writeFileSync('test-results.xml', xml);
        console.log('\nWrote test-results.xml');
    }

    process.exit(failed > 0 ? 1 : 0);
})();
