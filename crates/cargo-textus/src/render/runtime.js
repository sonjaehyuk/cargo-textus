// rustdoc가 만든 문서 DOM에 전역 렌더링을 적용한다. Markdown이나 수식 구문을 직접 파싱하지 않는다.
(() => {
    if (window.__textusRender) return;
    window.__textusRender = true;
    const config = __TEXTUS_CONFIG__;
    // 라이브러리 로딩 실패도 문서 본문은 그대로 남겨 읽을 수 있게 한다.
    const report = (error) => console.error("textus rendering:", error);
    __TEXTUS_ALERTS__
    const start = async () => {
        const root = document.querySelector('meta[name="rustdoc-vars"]')?.dataset.rootPath;
        if (root === undefined) { report("rustdoc root path is missing"); return; }
        const base = new URL(root + "textus-assets/", location.href);
        // classic script는 file://에서도 모듈 CORS 제약 없이 로드된다.
        const script = (name) => new Promise((resolve, reject) => {
            const element = document.createElement("script");
            element.src = new URL(name, base).href;
            element.onload = resolve;
            element.onerror = () => reject(new Error(`could not load ${name}`));
            document.head.append(element);
        });
        const style = (name) => {
            const element = document.createElement("link");
            element.rel = "stylesheet";
            element.href = new URL(name, base).href;
            document.head.append(element);
        };
        if (config.math) style("katex.css");
        if (config.alerts) style("alerts.css");
        for (const name of config.css) style(name);
        const docs = [...document.querySelectorAll(".docblock")];
        if (config.alerts) renderAlerts(docs);
        // 라이브러리별 실패를 분리하여 한 기능의 오류가 다른 기능을 막지 않게 한다.
        await Promise.all([
            (async () => {
                if (!config.mermaid) return;
                await script("mermaid.js");
                mermaid.initialize({startOnLoad: false, securityLevel: "strict"});
                // rustdoc의 language-mermaid 클래스만 연결한다. 내용 해석은 Mermaid가 맡는다.
                for (const doc of docs) {
                    for (const code of doc.querySelectorAll("pre.language-mermaid, code.language-mermaid")) {
                        const pre = code.closest("pre");
                        if (pre) { pre.classList.add("mermaid"); pre.textContent = code.textContent; }
                    }
                }
                await mermaid.run({querySelector: ".docblock .mermaid", suppressErrors: true});
            })().catch(report),
            (async () => {
                if (!config.math) return;
                await script("katex.js");
                await script("auto-render.js");
                for (const doc of docs) {
                    renderMathInElement(doc, {
                        delimiters: [
                            {left: "$$", right: "$$", display: true},
                            {left: "$", right: "$", display: false},
                        ],
                        ignoredClasses: ["mermaid", "katex", "textus-no-math"],
                        throwOnError: false,
                        trust: false,
                    });
                }
            })().catch(report),
        ]);
        // 사용자 자산은 내장 렌더링 처리 후 지정 순서대로 한 번씩 실행한다.
        for (const name of config.js) {
            try { await script(name); } catch (error) { report(error); }
        }
        document.documentElement.dataset.textusRendered = "true";
    };
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start, {once: true});
    else start();
})();
