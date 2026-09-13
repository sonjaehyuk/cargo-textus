// rustdoc의 최상위 인용문 DOM에만 GitHub 형식 알림을 적용한다.
// 본문 노드를 재생성하지 않아 링크·목록·강조·코드와 이벤트 연결을 보존한다.
const renderAlerts = (docs) => {
    // 외부 아이콘 폰트 없이 사용하는 자체 SVG 경로. 제목이 의미를 전달하므로 아이콘은 장식이다.
    const types = {
        NOTE: ["Note", "M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1ZM8 7v5M8 4v.5"],
        TIP: ["Tip", "M5.5 11C5.5 9 2.5 8 3 5a5 5 0 0 1 10 0c.5 3-2.5 4-2.5 6ZM5.5 13h5M6.5 15h3"],
        IMPORTANT: ["Important", "M2 1h12v10H7l-4 4v-4H2ZM8 4v3M8 9v.5"],
        WARNING: ["Warning", "M8 1 15 14H1ZM8 6v4M8 12v.5"],
        CAUTION: ["Caution", "M5 1h6l4 4v6l-4 4H5l-4-4V5ZM8 4v5M8 11v.5"],
    };
    for (const doc of docs) {
        for (const quote of doc.querySelectorAll(":scope > blockquote")) {
            if (quote.classList.contains("textus-alert")) continue;
            const paragraph = quote.firstElementChild;
            const text = paragraph?.firstChild;
            // 마커는 첫 문단의 첫 텍스트 줄 전체여야 한다. 인라인 코드나 강조 속 마커는 제외한다.
            if (paragraph?.tagName !== "P" || text?.nodeType !== Node.TEXT_NODE) continue;
            const match = text.data.match(/^\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\][ \t]*(?:\r?\n|$)/);
            if (!match) continue;
            const remainder = text.data.slice(match[0].length);
            // 같은 줄의 링크·강조를 마커 뒤 본문으로 오인하지 않는다. 명시적 br은 줄 경계다.
            if (!match[0].endsWith("\n") && text.nextSibling && text.nextSibling.nodeName !== "BR") continue;
            if (!remainder && text.nextSibling?.nodeName === "BR") text.nextSibling.remove();
            text.data = remainder;
            if (!paragraph.hasChildNodes() || (paragraph.childNodes.length === 1 && !text.data.trim())) paragraph.remove();
            const [label, path] = types[match[1]];
            const title = document.createElement("p");
            title.className = "textus-alert-title";
            const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
            icon.setAttribute("viewBox", "0 0 16 16");
            icon.setAttribute("aria-hidden", "true");
            icon.setAttribute("focusable", "false");
            const shape = document.createElementNS(icon.namespaceURI, "path");
            shape.setAttribute("d", path);
            icon.append(shape);
            title.append(icon, document.createTextNode(label));
            quote.classList.add("textus-alert", `textus-alert-${match[1].toLowerCase()}`);
            quote.prepend(title);
        }
    }
};
