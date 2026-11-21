const assert = require('node:assert/strict');

module.exports = async (page, name) => {
    const references = page.locator('sup[id] > a[aria-haspopup="dialog"]');
    const popup = page.locator('.textus-footnote-preview');
    const body = popup.locator('.textus-footnote-body');
    assert.equal(await references.count(), name === 'index.html' ? 3 : 1);
    const reference = references.first();
    const sourceId = (await reference.getAttribute('href')).slice(1);
    const original = await page.locator(`[id="${sourceId}"]`).innerHTML();
    await reference.hover();
    await popup.waitFor();
    assert.equal(await reference.getAttribute('aria-expanded'), 'true');
    assert.equal(await body.getAttribute('role'), 'dialog');
    assert.equal(await body.getAttribute('aria-modal'), null);
    assert.ok((await body.textContent()).includes(name === 'index.html' ? 'Footnote formatting' : 'page footnote'));
    assert.equal(await body.locator('a[href^="#fnref"]').count(), 0);
    assert.equal(await page.locator(`[id="${sourceId}"]`).innerHTML(), original);

    const insideViewport = async () => {
        const bounds = await popup.boundingBox();
        const size = page.viewportSize();
        assert.ok(bounds.x >= 7.5 && bounds.y >= 7.5, JSON.stringify(bounds));
        assert.ok(bounds.x + bounds.width <= size.width - 7.5, JSON.stringify(bounds));
        assert.ok(bounds.y + bounds.height <= size.height - 7.5, JSON.stringify(bounds));
    };
    await insideViewport();
    if (name !== 'index.html') {
        await page.keyboard.press('Escape');
        assert.equal(await popup.count(), 0);
        return;
    }

    assert.equal(await body.locator('strong').textContent(), 'formatting');
    assert.equal(await body.locator('a[href="https://example.com"]').textContent(), 'link');
    assert.ok(await body.locator('.katex').count());
    assert.ok(await body.locator('.mermaid svg').count());
    assert.ok(await body.locator('.mermaid .node rect').first().evaluate(el => getComputedStyle(el).fill !== 'rgb(0, 0, 0)'));
    const duplicateIds = await page.evaluate(() => {
        const ids = [...document.querySelectorAll('[id]')].map(el => el.id);
        return ids.filter((id, index) => ids.indexOf(id) !== index);
    });
    assert.deepEqual(duplicateIds, []);
    const missingSvgTargets = await body.locator('svg').evaluate(svg => [...svg.querySelectorAll('*')].flatMap(el =>
        [...el.attributes].flatMap(attr => [...attr.value.matchAll(/url\(["']?#([^\s)"']+)/g)]
            .filter(match => !svg.querySelector(`[id="${match[1]}"]`)).map(match => match[1]))));
    assert.deepEqual(missingSvgTargets, []);
    for (const theme of ['light', 'dark', 'ayu']) {
        await page.evaluate(theme => document.documentElement.dataset.theme = theme, theme);
        const background = await body.evaluate(el => getComputedStyle(el).backgroundColor);
        assert.equal(background, await page.locator('body').evaluate(el => getComputedStyle(el).backgroundColor));
    }

    // 같은 각주를 가리키는 링크가 여러 개여도 팝업은 하나만 유지한다.
    await page.keyboard.press('Escape');
    await references.nth(1).focus();
    assert.equal(await popup.count(), 1);
    assert.equal(await reference.getAttribute('aria-expanded'), 'false');
    assert.equal(await references.nth(1).getAttribute('aria-expanded'), 'true');
    await page.keyboard.press('Escape');
    await references.nth(1).evaluate(el => el.blur());
    await reference.focus();
    await page.keyboard.press('Tab');
    assert.ok(await body.evaluate(el => el === document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await body.locator('a[href="https://example.com"]').evaluate(el => el === document.activeElement));
    await page.keyboard.press('Tab');
    assert.ok(await references.nth(1).evaluate(el => el === document.activeElement));
    await page.keyboard.press('Escape');
    await references.nth(1).evaluate(el => el.blur());

    // 실제 rustdoc 링크를 네 모서리로 옮겨 좌우 전환과 상하 경계 보정을 확인한다.
    // 고정한 테스트 참조 자체도 viewport 좌표를 사용하게 한다. 원래 contain 배치는 위에서 검사했다.
    const layoutOverride = await page.addStyleTag({content: 'details.toggle { contain: none; }'});
    const place = async (target, x, y) => {
        await page.keyboard.press('Escape');
        await target.evaluate((el, {x, y}) => {
            el.blur();
            el.style.cssText = `position:fixed;left:${x}px;top:${y}px;z-index:101;line-height:20px`;
        }, {x, y});
        await page.mouse.move(1, 1);
        await target.hover();
        await popup.waitFor();
    };
    for (const [x, y] of [[16, 16], [1248, 16], [16, 680], [1248, 680]]) {
        await place(reference, x, y);
        await insideViewport();
        const anchor = await reference.boundingBox();
        const bounds = await popup.boundingBox();
        if (x > 1000) assert.ok(bounds.x + bounds.width <= anchor.x + 0.5);
        else assert.ok(bounds.x >= anchor.x + anchor.width - 0.5);
    }
    await place(reference, 40, 180);
    const anchor = await reference.boundingBox();
    const bounds = await popup.boundingBox();
    // 투명 여백을 천천히 지나가도 닫히지 않아 본문을 읽거나 링크를 사용할 수 있다.
    await page.mouse.move(bounds.x + 2, anchor.y + anchor.height / 2);
    await page.waitForTimeout(250);
    assert.equal(await popup.count(), 1);
    await body.hover();
    await page.waitForTimeout(250);
    assert.equal(await popup.count(), 1);
    await page.mouse.move(1, 1);
    await popup.waitFor({state: 'detached'});

    const long = references.last();
    await place(long, 1248, 240);
    await insideViewport();
    assert.ok(await body.evaluate(el => el.scrollHeight > el.clientHeight));
    const pageScroll = await page.evaluate(() => scrollY);
    await body.hover();
    await page.mouse.wheel(0, 360);
    await page.waitForFunction(() => document.querySelector('.textus-footnote-body')?.scrollTop > 0);
    assert.equal(await page.evaluate(() => scrollY), pageScroll);
    await page.waitForTimeout(250);
    assert.equal(await popup.count(), 1);

    // 늦게 추가된 넓은 콘텐츠도 가로 스크롤로 읽을 수 있고 화면 영역을 벗어나지 않는다.
    await body.evaluate(el => {
        const wide = document.createElement('div');
        wide.style.minWidth = '1200px';
        wide.textContent = 'wide footnote content';
        el.append(wide);
        el.scrollLeft = 200;
    });
    assert.ok(await body.evaluate(el => el.scrollLeft > 0));
    await insideViewport();

    // Tab으로 미리보기를 읽고 PageDown으로 스크롤하며 Escape로 참조에 돌아간다.
    await page.keyboard.press('Escape');
    await long.focus();
    await page.keyboard.press('Tab');
    assert.ok(await body.evaluate(el => el === document.activeElement));
    await page.keyboard.press('PageDown');
    await page.waitForFunction(() => document.querySelector('.textus-footnote-body')?.scrollTop > 0);
    await page.keyboard.press('Escape');
    assert.equal(await popup.count(), 0);
    assert.ok(await long.evaluate(el => el === document.activeElement));
    await page.keyboard.press('ArrowDown');
    assert.ok(await body.evaluate(el => el === document.activeElement));
    await page.keyboard.press('Shift+Tab');
    assert.ok(await long.evaluate(el => el === document.activeElement));
    await page.keyboard.press('Escape');

    // 페이지 휠 스크롤을 잠그지 않고 표시 중인 참조 위치를 따라간다.
    await page.evaluate(() => scrollTo(0, 0));
    await long.evaluate(el => el.blur());
    await long.focus();
    await page.mouse.move(500, 400);
    await page.mouse.wheel(0, 300);
    await page.waitForFunction(() => scrollY > 0);
    await insideViewport();

    // 좁고 낮은 화면에서는 상하 배치로 바꾸고 본문 높이를 제한한다.
    await page.keyboard.press('Escape');
    await page.setViewportSize({width: 320, height: 240});
    await place(long, 150, 100);
    await insideViewport();
    assert.ok(await body.evaluate(el => el.scrollHeight > el.clientHeight));
    await page.setViewportSize({width: 260, height: 200});
    await page.waitForFunction(() => {
        const rect = document.querySelector('.textus-footnote-preview').getBoundingClientRect();
        return rect.right <= innerWidth - 7.5 && rect.bottom <= innerHeight - 7.5;
    });
    await insideViewport();
    await page.emulateMedia({media: 'print'});
    assert.equal(await popup.isVisible(), false);
    await page.emulateMedia({media: 'screen'});
    await page.setViewportSize({width: 1280, height: 720});
    await page.keyboard.press('Escape');

    // 원래 각주와 돌아가기 링크는 클릭과 Enter로 계속 사용할 수 있다.
    await reference.evaluate(el => el.removeAttribute('style'));
    await long.evaluate(el => el.removeAttribute('style'));
    await layoutOverride.evaluate(el => el.remove());
    await reference.click();
    assert.equal(new URL(page.url()).hash, `#${sourceId}`);
    assert.equal(await popup.count(), 0);
    await page.locator(`[id="${sourceId}"] a[href="#fnref1"]`).click();
    assert.equal(new URL(page.url()).hash, '#fnref1');
    await page.keyboard.press('Escape');
    await reference.focus();
    await page.keyboard.press('Enter');
    assert.equal(new URL(page.url()).hash, `#${sourceId}`);
};
