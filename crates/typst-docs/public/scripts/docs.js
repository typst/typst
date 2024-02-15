const hoverQuery = window.matchMedia("(hover: hover)");
let isHover = hoverQuery.matches;
hoverQuery.addEventListener("change", (n) => {
  isHover = n.matches;
});
const hasTransitionEnd = "ontransitionend" in window,
  prefersReducedMotion = window.matchMedia(
    "(prefers-reduced-motion: reduce)"
  ).matches,
  collapseFactory = (n, t, s) => (o, e) => {
    t.removeEventListener("transitionend", s),
      e && (t.style.maxHeight = `${o}px`),
      window.requestAnimationFrame(() => {
        (t.style.pointerEvents = "none"),
          (t.style.userSelect = "none"),
          (t.style.maxHeight = "0px"),
          (t.style.opacity = "0");
      }),
      n.setAttribute("aria-expanded", "false");
  },
  expandFactory = (n, t) => {
    const s = () => {
      t.style.maxHeight = "none";
    };
    return {
      expand: (o, e) => {
        hasTransitionEnd && !prefersReducedMotion && e
          ? (t.addEventListener("transitionend", s, { once: !0 }),
            (t.style.maxHeight = `${o}px`),
            (t.style.pointerEvents = "auto"),
            (t.style.userSelect = "auto"),
            (t.style.opacity = "1"))
          : s(),
          n.setAttribute("aria-expanded", "true");
      },
      handler: s,
    };
  },
  toggleFactory = (n, t) => {
    const { expand: s, handler: o } = expandFactory(n, t),
      e = collapseFactory(n, t, o);
    return {
      toggle: (c) => {
        n.getAttribute("aria-expanded") === "true" ? e(c, !0) : s(c, !0);
      },
      expand: s,
      collapse: e,
    };
  };
function setUpAccordeon(n) {
  const t = Array.from(n.children),
    s = t.some((e) => e.querySelector("li > a[aria-current]"));
  let o = t
    .filter((e) => e.tagName === "LI")
    .reduce((e, c) => {
      const a = c.getElementsByTagName("button"),
        d = c.getElementsByTagName("ul"),
        p = c.getElementsByTagName("a");
      if (a.length === 0 || d.length === 0) return e;
      const f = d[0];
      return (
        e.push({
          button: a[0],
          child: f,
          target: c,
          anchor: p.length > 0 ? p[0] : null,
        }),
        e
      );
    }, []);
  return (
    o.length === 0 ||
      (o.forEach((e) => {
        const c = setUpAccordeon(e.child),
          a = e.anchor.getAttribute("aria-current") === "page";
        setUpSingleAccordeon(e.target, e.button, e.child, c || a, !1);
      }),
      n.classList.add("animated")),
    s
  );
}
function setUpSingleAccordeon(n, t, s, o, e = !1) {
  const { toggle: c, expand: a, collapse: d } = toggleFactory(n, s);
  s.style.overflow = "visible hidden";
  let p = s.offsetHeight;
  o ? a(p, !1) : d(p, !1),
    t.addEventListener("click", (f) => {
      e && f.preventDefault(), c(p);
    });
}
function isElementInViewport(n) {
  const t = n.getBoundingClientRect();
  return (
    t.top >= 0 &&
    t.left >= 0 &&
    t.bottom <= (window.innerHeight || document.documentElement.clientHeight) &&
    t.right <= (window.innerWidth || document.documentElement.clientWidth)
  );
}
function setUpOnPageNavigation(n) {
  const t = Array.from(n.querySelectorAll("li[data-assoc]")).reduce((o, e) => {
      const c = e.getAttribute("data-assoc"),
        a = e.getElementsByTagName("a"),
        d = document.getElementById(c);
      return (
        d && a.length > 0 && o.push({ item: e, assoc: d, anchor: a[0] }), o
      );
    }, []),
    s = () => {
      const o = t.find((e) => isElementInViewport(e.assoc));
      o &&
        (t.forEach((e) => e.item.removeAttribute("aria-current")),
        o.item.setAttribute("aria-current", "true"));
    };
  s(),
    window.addEventListener("scroll", s),
    window.addEventListener("resize", s);
}
function setUpTooltip(n) {
  const t = n.querySelector("svg"),
    s = n.querySelector("div[role='tooltip']");
  let o = !1;
  const e = t.getElementsByTagName("title")[0];
  let c = e.textContent,
    a = 0;
  const d = 256,
    p = () => {
      const r = window.innerWidth,
        l = t.getBoundingClientRect();
      l.left + d / 2 > r
        ? ((s.style.left = `${r - d - l.left - 32}px`),
          s.classList.add("mobile"))
        : l.left - d / 2 < 0
        ? ((s.style.left = `${-l.left + 32}px`), s.classList.add("mobile"))
        : ((s.style.left = "-120px"), s.classList.remove("mobile")),
        (o = !0),
        (e.innerHTML = ""),
        (s.style.display = "block"),
        window.requestAnimationFrame(() => {
          (s.style.opacity = "1"),
            (s.style.pointerEvents = "auto"),
            s.removeAttribute("aria-hidden");
        }),
        (a = Date.now());
    },
    f = () => {
      (e.innerHTML = c), (o = !1);
      const r = () => {
        (s.style.display = "none"),
          (s.style.pointerEvents = "none"),
          s.setAttribute("aria-hidden", "true");
      };
      hasTransitionEnd && !prefersReducedMotion
        ? s.addEventListener("transitionend", r, { once: !0 })
        : r(),
        window.requestAnimationFrame(() => {
          (s.style.opacity = "0"),
            hasTransitionEnd &&
              !prefersReducedMotion &&
              s.removeEventListener("transitionend", r);
        });
    };
  t.addEventListener("click", () => {
    isHover || (o && Date.now() - a > 100 ? f() : p());
  }),
    t.addEventListener("mouseenter", (r) => {
      r.preventDefault(), p();
    }),
    t.addEventListener("mousemove", (r) => {
      o && r.preventDefault();
    }),
    t.addEventListener("focus", p),
    t.addEventListener("blur", f),
    t.addEventListener("mouseleave", f),
    window.addEventListener("click", (r) => {
      o && !n.contains(r.target) && f();
    }),
    window.addEventListener("mouseleave", f),
    window.addEventListener("keydown", (r) => {
      o && r.key === "Escape" && f();
    });
}
function setUpCollapsingSidebar(n) {
  const t = document.querySelector("button.hamburger"),
    s = n.querySelector("button.close");
  n.classList.add("mobile-hidden"), (n.style.opacity = "1");
  const o = (d) => {
      n.classList.contains("mobile-hidden") ||
        n.contains(d.target) ||
        (c(), d.preventDefault());
    },
    e = () => {
      n.classList.add("mobile-hidden"), (n.style.transform = "translateX(0)");
    },
    c = () => {
      window.removeEventListener("click", o),
        hasTransitionEnd && !prefersReducedMotion
          ? ((n.style.transform = "translateX(-100%)"),
            n.addEventListener("transitionend", e, { once: !0 }))
          : e();
    },
    a = () => {
      n.removeEventListener("transitionend", e),
        (n.style.transform = "translateX(-100%)"),
        n.classList.remove("mobile-hidden"),
        requestAnimationFrame(() => {
          window.addEventListener("click", o),
            (n.style.transform = "translateX(0)");
        });
    };
  t.addEventListener("click", a), s.addEventListener("click", c);
}
function copyText(n) {
  if ("clipboard" in navigator) {
    navigator.clipboard.writeText(n);
    return;
  } else {
    const t = document.createElement("input");
    (t.value = n),
      document.body.appendChild(t),
      t.select(),
      document.execCommand("copy"),
      document.body.removeChild(t);
  }
}
function setUpSymbolFlyout(n) {
  const t = document.getElementById("flyout-template"),
    s = document.getElementById("flyout-sym-row");
  if (!n || !t || !s) return;
  if (!("content" in document.createElement("template"))) {
    console.warn("Browser does not support template elements");
    return;
  }
  const o = t.content.firstElementChild.cloneNode(!0);
  (o.style.display = "none"), n.appendChild(o);
  const e = o.querySelector(".info button"),
    c = o.querySelector(".info button .sym"),
    a = o.querySelector(".sym-name code"),
    d = o.querySelector(".info .unic-name"),
    p = o.querySelector(".info .codepoint span"),
    f = o.querySelector(".info .accent"),
    r = o.querySelector(".info .accent img"),
    l = o.querySelector(".variants-box"),
    h = o.querySelector(".variants-box .symbol-grid"),
    m = o.querySelector(".shorthand"),
    i = m.querySelector(".remark"),
    y = o.querySelector(".shorthand code"),
    E = o.querySelector(".sym-name .copy"),
    N = o.querySelector(".shorthand .copy"),
    T = o.querySelector(".codepoint .copy"),
    P = "/assets/icons/16-close.svg",
    H = "/assets/icons/16-check.svg";
  let g;
  const C = () => {
      const u = document.getElementById(g);
      u && (u.ariaHasPopup = "false"),
        (g = void 0),
        (o.style.display = "none"),
        window.removeEventListener("click", k);
      for (const { target: w, listener: v } of L)
        w.removeEventListener("click", v);
    },
    k = (u) => {
      g !== void 0 &&
        g !== u.target.id &&
        !o.contains(u.target) &&
        (C(), u.preventDefault());
    };
  let L = [];
  const x = (u) => {
    (u.ariaHasPopup = "true"), (g = u.id), (o.style.display = "block");
    const w = u.id.replace(/^symbol-/, ""),
      v = u.dataset.unicName,
      A = parseInt(u.dataset.codepoint, 10),
      b = u.dataset.accent != null,
      X = u.dataset.alternates ? u.dataset.alternates.split(" ") : [],
      F = u.dataset.mathShorthand,
      U = u.dataset.markupShorthand,
      S = U || F,
      M = String.fromCodePoint(A),
      $ = u.dataset.override;
    copyText(M);
    let q = A.toString(16).toUpperCase();
    q.length < 4 && (q = "0".repeat(4 - q.length) + q),
      o.classList.toggle("override", $ != null),
      (c.textContent = $ ?? M),
      (a.textContent = w),
      (d.textContent = v),
      (p.textContent = q),
      (f.style.display = b ? "block" : "none"),
      (r.src = b ? H : P),
      r.setAttribute("alt", b ? "Yes" : "No"),
      (m.style.display = S && S.length > 0 ? "block" : "none"),
      (y.textContent = S);
    let O = () => {
      copyText(w);
    };
    if (
      (E.addEventListener("click", O),
      L.push({ target: E, listener: O }),
      S && S.length > 0)
    ) {
      let B = () => {
        copyText(S);
      };
      N.addEventListener("click", B),
        L.push({ target: N, listener: B }),
        i &&
          (F && !U
            ? ((i.textContent = "(only in math)"), (i.style.display = "inline"))
            : !F && U
            ? ((i.textContent = "(only in markup)"),
              (i.style.display = "inline"))
            : (i.style.display = "none"));
    }
    let R = () => {
      copyText("\\u{" + q + "}");
    };
    if (
      (T.addEventListener("click", R),
      L.push({ target: T, listener: R }),
      l !== null && h !== null)
    ) {
      const B = X.map((W) => {
        const D = s.content.firstElementChild.cloneNode(!0),
          Q = D.querySelector(".sym"),
          j = D.querySelector("button"),
          I = document.getElementById(`symbol-${W}`),
          K = I?.dataset.override;
        if (I)
          j.classList.toggle("override", K != null),
            (Q.textContent =
              K ?? String.fromCodePoint(parseInt(I.dataset.codepoint, 10)));
        else return null;
        return (
          j.addEventListener("click", (Y) => {
            x(I), I.scrollIntoView(), Y.preventDefault();
          }),
          D
        );
      }).filter((W) => W != null);
      (l.style.display = B.length > 0 ? "block" : "none"),
        h.replaceChildren(...B);
    }
    let V = u.offsetLeft - 12;
    const z = u.offsetTop - 12;
    u.getBoundingClientRect().left + 408 > window.innerWidth &&
      (V = Math.max(
        8,
        window.innerWidth - n.getBoundingClientRect().left - 424
      )),
      (o.style.left = `${V}px`),
      (o.style.top = `${z}px`);
  };
  e.addEventListener("click", C),
    window.addEventListener("keydown", (u) => {
      u.key === "Escape" && g !== void 0 && (C(), u.preventDefault());
    });
  for (let u = 0; u < n.children.length; u++) {
    const w = n.children[u];
    w.tagName === "LI" &&
      w.addEventListener("click", (v) => {
        x(w), v.preventDefault();
      });
  }
  window.requestAnimationFrame(() => {
    window.removeEventListener("click", k),
      window.addEventListener("click", k, { capture: !0 });
  });
}
function setUpSymbolSearch() {
  const n = document.querySelector("main > .symbol-grid"),
    t = document.getElementById("symbol-search");
  if (!n || !t) return;
  const s = Array.from(n.children).filter((p) => p.tagName === "LI");
  s.forEach((p, f) => {
    p.dataset.originalOrder = f;
  });
  const o = s.map((p) => {
      const f = parseInt(p.dataset.codepoint, 10);
      return {
        name: p.id.replace(/^symbol-/, ""),
        unicName: p.dataset.unicName,
        symbol: String.fromCodePoint(f),
        shorthand: p.dataset.shorthand,
      };
    }),
    e = new Fuse(o, {
      keys: [
        { name: "symbol", weight: 0.95 },
        { name: "name", weight: 0.9 },
        { name: "unicName", weight: 0.7 },
        { name: "shorthand", weight: 0.5 },
      ],
      threshold: 0.3,
      ignoreLocation: !0,
    }),
    c = () => {
      if (t.value.length === 0) {
        s.forEach((r) => (r.style.display = "block")),
          s
            .sort(
              (
                { dataset: { originalOrder: r } },
                { dataset: { originalOrder: l } }
              ) => parseInt(r, 10) - parseInt(l, 10)
            )
            .forEach((r) => r.parentNode.appendChild(r));
        return;
      }
      const p = e.search(t.value).map((r) => r.item.name),
        f = parseInt(
          t.value.replace(/^\\?[uU]\{?\+?/, "").replace(/\}$/, ""),
          16
        ).toString();
      s.forEach((r) => {
        const l = p.indexOf(r.id.replace(/^symbol-/, ""));
        l !== -1 || r.dataset.codepoint == f
          ? ((r.style.display = "block"), (r.dataset.order = l))
          : ((r.style.display = "none"), delete r.dataset.order);
      }),
        s
          .filter((r) => "order" in r.dataset)
          .sort(
            ({ dataset: { order: r } }, { dataset: { order: l } }) =>
              parseInt(r, 10) - parseInt(l, 10)
          )
          .forEach((r) => r.parentNode.appendChild(r));
    };
  t.addEventListener("input", c), t.addEventListener("keydown", c);
  const d = new URLSearchParams(window.location.search).get("query");
  d && (t.value = d), c();
}
async function setUpSearch(n, t) {
  const s = new AbortController(),
    o = setTimeout(() => s.abort(), 1e4),
    e = await fetch("/assets/search.json?bust=20230915", { signal: s.signal })
      .then((a) => (a.ok ? a.json() : null))
      .catch((a) => (console.error(a), null));
  if ((clearTimeout(o), !e)) return;
  const c = () => {
    let a = [],
      d = [];
    const p = n.value,
      f = n.value
        .toLowerCase()
        .split(/[!-/:-@[-`{-~\s]/g)
        .filter((l) => l.length > 0);
    function r(l) {
      if (a.includes(l)) return;
      const h = e.items[l];
      if (((d[l] = score(h, p)), a.length < 10)) {
        a.push(l);
        return;
      }
      let m = 0;
      for (let i = 0; i < a.length; i++) d[a[i]] < d[a[m]] && (m = i);
      if (d[l] > d[a[m]]) {
        a[m] = l;
        return;
      }
    }
    for (const l of f) {
      let h = 0,
        m = e.words.length - 1;
      for (; h < m; ) {
        let y = Math.floor((h + m) / 2);
        if (e.words[y].startsWith(l)) {
          h = y;
          break;
        } else l < e.words[y] ? (m = y - 1) : (h = y + 1);
      }
      let i = h;
      for (; i > 0 && e.words[i].startsWith(l); ) i--;
      for (i++; i < e.words.length && e.words[i].startsWith(l); ) {
        for (const y of e.hits[i]) r(y);
        i++;
      }
    }
    a.sort((l, h) => d[h] - d[l]),
      t.replaceChildren(
        ...a.map((l) => {
          const h = e.items[l];
          let m = h.route;
          h.kind == "Symbols" && (m += "?query=" + p);
          const i = document.createElement("li"),
            y = document.createElement("a"),
            E = document.createElement("span");
          return (
            (y.href = m),
            (y.textContent = h.title),
            E.classList.add("type"),
            (E.textContent = h.kind),
            y.appendChild(E),
            i.appendChild(y),
            i
          );
        })
      ),
      t.classList.toggle("hidden", a.length === 0);
  };
  n.addEventListener("keydown", c), n.addEventListener("keyup", c), c();
}
const FACTORS = {
  Type: 1,
  Function: 0.9,
  Parameter: 0.8,
  Method: 0.7,
  Symbols: 0.6,
  Chapter: 0.5,
  Category: 0.5,
};
function score(n, t) {
  return (
    (FACTORS[n.kind.split(" ")[0]] || 0.4) *
    Math.max(
      stringScore(n.title, t),
      ...(n.keywords || []).map((o) => stringScore(o, t))
    )
  );
}
function stringScore(n, t) {
  const s = simplify(n),
    o = simplify(t);
  return s.includes(o) ? 100 - n.length : 0;
}
function simplify(n) {
  return n.toLowerCase().replaceAll(/[!-/:-@[-`{-~\s]/g, "");
}
async function setUpPackages() {
  const n = document.querySelector("main > .package-list > tbody"),
    t = document.getElementById("package-search"),
    s = document.getElementById("package-show-all-versions");
  if (!n || !t) return;
  const e = await (
    await fetch("https://packages.typst.org/preview/index.json")
  ).json();
  let c,
    a = [],
    d = [],
    p;
  function f() {
    const m = s.checked;
    if (p !== m) {
      (p = m), (d = []);
      for (let i = 0; i < e.length; i++)
        (m || i + 1 >= e.length || e[i].name !== e[i + 1].name) && d.push(e[i]);
      c = new Fuse(d, {
        keys: [
          { name: "name", weight: 0.9 },
          { name: "keywords", weight: 0.8 },
          { name: "description", weight: 0.6 },
        ],
        threshold: 0.3,
        ignoreLocation: !0,
      });
    }
  }
  function r() {
    f();
    for (const { target: i, listener: y } of a)
      i.removeEventListener("click", y);
    a = [];
    let m;
    t.value.length === 0 ? (m = d) : (m = c.search(t.value).map((i) => i.item)),
      n.replaceChildren();
    for (let i = 0; i < m.length; i++) {
      const {
          name: y,
          version: E,
          description: N,
          repository: T,
          homepage: P,
        } = m[i],
        H = P || T,
        g = document.createElement("tr"),
        C = () => {
          copyText(`#import "@preview/${y}:${E}"`);
        },
        k = document.createElement("td"),
        L = document.createElement("a");
      (L.href = `https://github.com/typst/packages/tree/main/packages/preview/${y}/${E}`),
        (L.textContent = y),
        k.appendChild(L),
        g.appendChild(k);
      const x = document.createElement("td");
      (x.textContent = N), g.appendChild(x);
      const u = document.createElement("td");
      (u.textContent = E), g.appendChild(u);
      const w = document.createElement("td"),
        v = document.createElement("button");
      v.classList.add("copy"),
        (v.innerHTML =
          '<img width="16" height="16" alt="Copy" src="/assets/icons/16-copy.svg">'),
        v.addEventListener("click", C),
        w.appendChild(v),
        a.push({ target: v, listener: C }),
        g.appendChild(w);
      const A = document.createElement("td");
      if (H) {
        const b = document.createElement("a");
        (b.href = H),
          (b.innerHTML =
            '<img width="16" height="16" alt="Copy" src="/assets/icons/16-link.svg">'),
          A.appendChild(b);
      }
      g.appendChild(A), n.appendChild(g);
    }
  }
  t.addEventListener("input", r),
    t.addEventListener("keydown", r),
    s.addEventListener("input", r);
  const h = new URLSearchParams(window.location.search).get("query");
  h && (t.value = h), r();
}
document.addEventListener("DOMContentLoaded", () => {
  for (const e of document.querySelectorAll("nav.folding > ul"))
    setUpAccordeon(e);
  const n = document.querySelector("#page-overview > ul");
  n && setUpOnPageNavigation(n),
    setUpCollapsingSidebar(document.querySelector("nav.folding"));
  for (const e of document.querySelectorAll("div.tooltip-context"))
    setUpTooltip(e);
  const t = document.querySelector(".page-end-buttons a.previous"),
    s = document.querySelector(".page-end-buttons a.next");
  window.addEventListener("keydown", (e) => {
    document.querySelectorAll("textarea:focus, input:focus").length > 0 ||
      (!e.metaKey &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.shiftKey &&
        (e.key === "ArrowLeft" && t
          ? (window.location.href = t.href)
          : e.key === "ArrowRight" && s && (window.location.href = s.href)));
  });
  for (const e of document.querySelectorAll(".previewed-code > pre"))
    e.clientWidth < e.scrollWidth && e.classList.add("big");
  setUpPackages(),
    setUpSearch(
      document.getElementById("docs-search"),
      document.getElementById("search-results")
    );
  const o = () => {
    for (const e of document.querySelectorAll("main > .symbol-grid"))
      setUpSymbolFlyout(e);
    setUpSymbolSearch();
  };
  window.requestIdleCallback ? window.requestIdleCallback(o) : o();
});
