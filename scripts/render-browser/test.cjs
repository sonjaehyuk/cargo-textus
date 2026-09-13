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
        fs.readFile(file, (error, data) => { res.statusCode = error ? 404 : 200; res.end(error ? '' : new URL(req.url, 'http://localhost').searchParams.has('alerts-off') && path.extname(file) === '.html' ? data.toString().replace('"alerts":true', '"alerts":false') : data); });
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
                const alerts = page.locator('.docblock .textus-alert');
                assert.equal(await alerts.count(), name === 'index.html' ? 6 : 1);
                assert.equal(await alerts.locator('.textus-alert-title svg[aria-hidden="true"]').count(), await alerts.count());
                if (name === 'index.html') {
                    assert.deepEqual(await alerts.locator('.textus-alert-title').allTextContents(), ['Note', 'Tip', 'Important', 'Warning', 'Caution', 'Note']);
                    assert.equal(await alerts.first().locator('strong').textContent(), 'information');
                    assert.equal(await alerts.first().locator('a').getAttribute('href'), 'https://example.com');
                    assert.equal(await page.locator('.textus-alert-important li').count(), 2);
                    assert.equal(await page.locator('.textus-alert-tip code').textContent(), 'code');
                    assert.ok((await alerts.last().textContent()).includes('Separate paragraph.'));
                    const ordinary = await page.locator('.docblock blockquote:not(.textus-alert)').allTextContents();
                    assert.ok(ordinary.some(text => text.includes('[!UNKNOWN]')));
                    assert.ok(ordinary.some(text => text.includes('[!NOTE] Same-line')));
                    assert.ok(ordinary.some(text => text.includes('A code marker')));
                    assert.equal(await page.locator('blockquote blockquote.textus-alert').count(), 0);
                    const color = async () => alerts.first().evaluate(el => getComputedStyle(el).borderLeftColor);
                    await page.evaluate(() => document.documentElement.dataset.theme = 'light');
                    const light = await color();
                    await page.evaluate(() => document.documentElement.dataset.theme = 'dark');
                    assert.notEqual(await color(), light);
                    await page.evaluate(() => document.documentElement.dataset.theme = 'ayu');
                    assert.notEqual(await color(), light);
                }
                assert.deepEqual(errors, []);
                assert.deepEqual(external, []);
                await page.close();
            }
        }
        // 실제 생성 헤더의 토글만 변경해 비활성 런타임이 원래 인용문을 보존하는지 검사한다.
        const disabled = await browser.newPage();
        await disabled.goto(`http://127.0.0.1:${server.address().port}/textus_render_demo/index.html?alerts-off`);
        await disabled.waitForFunction(() => document.documentElement.dataset.textusRendered === 'true');
        assert.equal(await disabled.locator('.textus-alert').count(), 0);
        assert.ok((await disabled.locator('.docblock').first().textContent()).includes('[!NOTE]'));
        assert.equal(await disabled.locator('link[href$="alerts.css"]').count(), 0);
        await disabled.close();
        console.log('Whole-page rendering: SVG, math, CSS/JS, alerts (enabled/disabled), themes, nested pages, file/HTTP and no external requests passed.');
    } finally {
        if (browser) await browser.close();
        await new Promise(resolve => server.close(resolve));
    }
})().catch(error => { console.error(error); process.exitCode = 1; });
