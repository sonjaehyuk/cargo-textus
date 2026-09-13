const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const http = require('node:http');
const fs = require('node:fs');

(async () => {
    const root = path.resolve(process.argv[2] || 'target/textus/render/textus-render-demo/default/doc');
    const server = http.createServer((req, res) => {
        const file = path.join(root, decodeURIComponent(new URL(req.url, 'http://localhost').pathname));
        const types = {'.js': 'text/javascript', '.css': 'text/css', '.html': 'text/html', '.woff2': 'font/woff2'};
        res.setHeader('Content-Type', types[path.extname(file)] || 'application/octet-stream');
        fs.readFile(file, (error, data) => { res.statusCode = error ? 404 : 200; res.end(error ? '' : data); });
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    let browser;
    try {
        browser = await chromium.launch({headless: true, args: ['--no-sandbox']});
        for (const protocol of ['file', 'http']) {
            for (const name of ['index.html', 'fn.example.html', 'nested/struct.Example.html']) {
                const page = await browser.newPage();
                const errors = [];
                const external = [];
                page.on('pageerror', e => errors.push(e.message));
                await page.route('**/*', route => {
                    const url = new URL(route.request().url());
                    if (url.protocol !== 'file:' && url.hostname !== '127.0.0.1') {
                        external.push(url.href); return route.abort();
                    }
                    return route.continue();
                });
                const relative = 'textus_render_demo/' + name;
                const url = protocol === 'file' ? pathToFileURL(path.join(root, relative)).href
                    : `http://127.0.0.1:${server.address().port}/${relative}`;
                await page.goto(url);
                await page.waitForFunction(() => document.documentElement.dataset.textusRendered === 'true');
                assert.ok(await page.locator('.docblock .mermaid svg').count(), `${protocol} ${name}: no SVG`);
                assert.ok(await page.locator('.docblock .katex').count(), `${protocol} ${name}: no math`);
                assert.equal(await page.locator('html').getAttribute('data-textus-custom'), 'loaded');
                assert.equal(await page.locator('.docblock').first().evaluate(el => getComputedStyle(el).getPropertyValue('--textus-example').trim()), 'enabled');
                if (name === 'index.html') {
                    assert.equal(await page.locator('.katex-display').count(), 1);
                    assert.ok(await page.locator('code').allTextContents().then(items => items.includes('$not_math$')));
                }
                assert.deepEqual(errors, []);
                assert.deepEqual(external, []);
                await page.close();
            }
        }
        console.log('Whole-page rendering: SVG, math, CSS/JS, nested pages, file/HTTP and no external requests passed.');
    } finally {
        if (browser) await browser.close();
        await new Promise(resolve => server.close(resolve));
    }
})().catch(error => { console.error(error); process.exitCode = 1; });
