// rustdoc의 각주 DOM만 연결한다. 원래 링크와 각주 본문은 그대로 두며 Markdown을 해석하지 않는다.
const renderFootnotes = (docs) => {
    // fragment를 CSS 선택자로 사용하지 않아 특수 문자나 잘못된 escape도 안전하게 건너뛴다.
    const fragment = (href) => {
        if (!href?.startsWith("#")) return null;
        try { return decodeURIComponent(href.slice(1)); } catch { return null; }
    };
    // 각주 정의를 가리키는 본문 sup만 대상으로 삼아 일반 앵커와 되돌아가기 링크를 제외한다.
    const references = new Map();
    for (const doc of docs) {
        for (const reference of doc.querySelectorAll('sup[id] > a[href^="#"]')) {
            const id = fragment(reference.getAttribute("href"));
            const source = id && document.getElementById(id);
            if (source?.matches(".footnotes > ol > li") && !reference.closest(".footnotes")) {
                references.set(reference, source);
            }
        }
    }
    if (!references.size) return;
    // 한 페이지에 팝업 하나만 유지한다. 식별자는 사용자 문서의 기존 ID와 충돌하지 않게 고른다.
    const referenceIds = new Set([...references.keys()].map(reference => reference.parentElement.id));
    const popup = document.createElement("div");
    popup.className = "textus-footnote-preview";
    const body = document.createElement("div");
    body.className = "docblock textus-footnote-body";
    body.setAttribute("role", "dialog");
    body.tabIndex = 0;
    let serial = 0;
    // 복제된 SVG ID도 사용자 문서의 임의 ID와 겹치지 않도록 같은 발급기를 사용한다.
    const uniqueId = () => {
        let id;
        do { id = `textus-footnote-${++serial}`; } while (document.getElementById(id));
        return id;
    };
    body.id = uniqueId();
    popup.append(body);
    // 활성 참조와 지연 닫기·재배치 작업은 이 초기화 인스턴스에서만 관리한다.
    let active = null;
    let closeTimer;
    let frame;

    // 렌더링된 서식을 복제하고 되돌아가기 링크만 제거한다. 원본의 ID와 SVG 참조는 분리한다.
    const copyContent = (source) => {
        const copy = source.cloneNode(true);
        for (const link of copy.querySelectorAll('a[href^="#"]')) {
            if (!referenceIds.has(fragment(link.getAttribute("href")))) continue;
            const parent = link.parentElement;
            link.remove();
            if (parent.tagName === "SUP" && !parent.textContent.trim()) parent.remove();
        }
        const ids = new Map();
        for (const element of copy.querySelectorAll("[id]")) {
            const id = uniqueId();
            ids.set(element.id, id);
            element.id = id;
        }
        // Mermaid의 SVG 내부 스타일과 url(#...) 연결도 새 ID를 사용해야 그림이 유지된다.
        const urls = (value) => value.replace(/url\((["']?)#([^\s)"']+)\1\)/g,
            (match, quote, id) => ids.has(id) ? `url(${quote}#${ids.get(id)}${quote})` : match);
        for (const element of copy.querySelectorAll("*")) {
            for (const attribute of [...element.attributes]) {
                let value = urls(attribute.value);
                if (["href", "xlink:href"].includes(attribute.name)) {
                    const id = fragment(value);
                    if (ids.has(id)) value = `#${ids.get(id)}`;
                } else if (["aria-labelledby", "aria-describedby", "aria-controls", "headers", "for"].includes(attribute.name)) {
                    value = value.split(/\s+/).map(id => ids.get(id) || id).join(" ");
                }
                if (value !== attribute.value) element.setAttribute(attribute.name, value);
            }
            if (element.tagName.toLowerCase() === "style") {
                let css = urls(element.textContent);
                for (const [oldId, newId] of ids) {
                    const escaped = CSS.escape(oldId).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
                    css = css.replace(new RegExp(`#${escaped}(?![\\w-])`, "g"), `#${newId}`);
                }
                element.textContent = css;
            }
        }
        body.replaceChildren(...copy.childNodes);
    };

    // 팝업 안에 있던 포커스만 원래 링크로 돌린다. 본문 스크롤이나 다른 포커스는 건드리지 않는다.
    const close = () => {
        clearTimeout(closeTimer);
        if (!active) return;
        if (popup.contains(document.activeElement)) active.focus({preventScroll: true});
        active.setAttribute("aria-expanded", "false");
        active.removeAttribute("aria-controls");
        active = null;
        popup.remove();
    };

    // 확대·모바일 키보드까지 고려한 보이는 영역 안에서 오른쪽, 왼쪽, 위/아래 순으로 배치한다.
    // 투명 padding이 참조와 본문 사이를 이어 주므로 느리게 포인터를 옮겨도 닫히지 않는다.
    const position = () => {
        if (!active) return;
        const view = window.visualViewport;
        const left = (view?.offsetLeft || 0) + 8;
        const top = (view?.offsetTop || 0) + 8;
        const right = left + (view?.width || document.documentElement.clientWidth) - 16;
        const bottom = top + (view?.height || document.documentElement.clientHeight) - 16;
        const anchor = active.getBoundingClientRect();
        if (!active.isConnected || !anchor.width || !anchor.height ||
            anchor.bottom < top || anchor.top > bottom || anchor.right < left || anchor.left > right) {
            close();
            return;
        }
        const width = Math.min(416, right - left);
        popup.style.width = `${width}px`;
        body.style.maxHeight = `${Math.max(0, bottom - top - 16)}px`;
        let x;
        let y;
        if (right - anchor.right >= width || anchor.left - left >= width) {
            x = right - anchor.right >= width ? anchor.right : anchor.left - width;
            y = Math.max(top, Math.min(anchor.top - 8, bottom - popup.getBoundingClientRect().height));
        } else {
            const below = Math.max(0, bottom - anchor.bottom);
            const above = Math.max(0, anchor.top - top);
            const useBelow = below >= above;
            body.style.maxHeight = `${Math.max(0, (useBelow ? below : above) - 16)}px`;
            x = Math.max(left, Math.min(anchor.left - 8, right - width));
            y = useBelow ? anchor.bottom : anchor.top - popup.getBoundingClientRect().height;
        }
        popup.style.left = `${x}px`;
        popup.style.top = `${y}px`;
    };

    // 스크롤 이벤트 빈도와 무관하게 다음 프레임에서 한 번만 위치를 갱신한다.
    const schedulePosition = () => {
        if (frame || !active) return;
        frame = requestAnimationFrame(() => { frame = null; position(); });
    };
    // 포인터와 포커스가 모두 떠난 경우에만 닫아 각주 표시에서 본문으로 이동할 시간을 준다.
    const scheduleClose = () => {
        clearTimeout(closeTimer);
        closeTimer = setTimeout(() => {
            if (active && !active.matches(":hover") && !popup.matches(":hover") &&
                document.activeElement !== active && !popup.contains(document.activeElement)) close();
        }, 180);
    };
    // 열 때마다 복제하므로 늦게 로드된 그림 등도 원본 문서의 현재 상태를 따른다.
    const show = (reference) => {
        clearTimeout(closeTimer);
        if (active === reference) return;
        close();
        copyContent(references.get(reference));
        active = reference;
        body.setAttribute("aria-label", `Footnote ${reference.textContent.trim()}`);
        reference.setAttribute("aria-expanded", "true");
        reference.setAttribute("aria-controls", body.id);
        // rustdoc details의 contain: layout이 fixed 좌표계를 바꾸므로 body 바로 아래에 둔다.
        document.body.append(popup);
        body.scrollTop = 0;
        body.scrollLeft = 0;
        position();
    };
    for (const reference of references.keys()) {
        reference.setAttribute("aria-haspopup", "dialog");
        reference.setAttribute("aria-expanded", "false");
        reference.setAttribute("aria-keyshortcuts", "ArrowDown");
        reference.addEventListener("pointerenter", event => {
            if (event.pointerType !== "touch") show(reference);
        });
        reference.addEventListener("pointerleave", scheduleClose);
        reference.addEventListener("focus", () => show(reference));
        reference.addEventListener("blur", scheduleClose);
        // 클릭·Enter·터치는 기존 각주로 이동하는 링크 동작을 그대로 실행한다.
        reference.addEventListener("click", close);
        reference.addEventListener("keydown", event => {
            if (event.key !== "ArrowDown" && !(event.key === "Tab" && !event.shiftKey && active === reference)) return;
            show(reference);
            body.focus({preventScroll: true});
            event.preventDefault();
        });
    }
    // 화면 최상위에 둔 팝업도 참조 다음의 Tab 순서로 연결한다. 마지막 요소를 지나면
    // 참조로 포커스를 돌리고 기본 Tab 동작을 허용하여 원래 문서의 다음 링크로 이어진다.
    body.addEventListener("keydown", event => {
        if (event.key !== "Tab" || !active) return;
        if (event.shiftKey && document.activeElement === body) {
            active.focus({preventScroll: true});
            event.preventDefault();
        } else if (!event.shiftKey) {
            const focusable = [...body.querySelectorAll('a[href], button, input, select, textarea, [tabindex]')]
                .filter(element => element.tabIndex >= 0 && !element.disabled && element.getClientRects().length);
            if (document.activeElement === (focusable.at(-1) || body)) close();
        }
    });
    popup.addEventListener("pointerenter", () => clearTimeout(closeTimer));
    popup.addEventListener("pointerleave", scheduleClose);
    popup.addEventListener("focusin", () => clearTimeout(closeTimer));
    popup.addEventListener("focusout", scheduleClose);
    document.addEventListener("keydown", event => {
        if (event.key === "Escape" && active) { close(); event.preventDefault(); }
    });
    document.addEventListener("pointerdown", event => {
        if (active && !active.contains(event.target) && !popup.contains(event.target)) close();
    });
    // 휠이나 터치 이벤트를 취소하지 않아 페이지와 팝업의 기본 스크롤을 모두 허용한다.
    document.addEventListener("scroll", event => {
        if (!popup.contains(event.target)) schedulePosition();
    }, {capture: true, passive: true});
    window.addEventListener("resize", schedulePosition);
    window.visualViewport?.addEventListener("resize", schedulePosition);
    window.visualViewport?.addEventListener("scroll", schedulePosition);
    // 이미지·폰트 로딩으로 본문 크기가 변해도 위치를 다시 계산한다.
    const observer = new ResizeObserver(schedulePosition);
    observer.observe(body);
};
