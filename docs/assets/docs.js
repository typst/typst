"use strict";

// The following line is replaced to include the docs base path.
const assetBase = "/assets/";
const closeIconSrc = assetBase + "16-close.svg";
const checkIconSrc = assetBase + "16-check.svg";
const searchIndexSrc = assetBase + "search.json";

const hasTransitionEnd = "ontransitionend" in window;
const prefersReducedMotion = window.matchMedia(
  "(prefers-reduced-motion: reduce)",
).matches;

const hoverQuery = window.matchMedia("(hover: hover)");
let isHover = hoverQuery.matches;
hoverQuery.addEventListener("change", (e) => {
  isHover = e.matches;
});

/**
 * Called at the bottom. Should hold all actual logic (no top-level set up
 * code).
 */
function main() {
  document.addEventListener("DOMContentLoaded", () => {
    setUpKeyboardNavigation();
    setUpCollapsingSidebar();
    setUpFoldingNav();
    setUpOnThisPage();
    setUpTooltips();
    setUpPreviewSplits();
    setUpSymbolFlyouts();
    setUpGlobalSearch();
    setUpSymbolSearch();
  });
}

/**
 * Sets up keyboard navigation:
 *
 * - Left arrow to go to the previous docs page
 * - Right arrow to go to the next docs page
 */
function setUpKeyboardNavigation() {
  /** @type {HTMLAnchorElement | null} */
  const prevBtn = document.querySelector(".page-end-buttons a.previous");
  /** @type {HTMLAnchorElement | null} */
  const nextBtn = document.querySelector(".page-end-buttons a.next");

  window.addEventListener("keyup", (e) => {
    const inputs = document.querySelectorAll("textarea:focus, input:focus");
    if (inputs.length > 0) {
      return;
    }

    if (!e.metaKey && !e.ctrlKey && !e.altKey && !e.shiftKey) {
      if (e.key === "ArrowLeft" && prevBtn) {
        window.location.href = prevBtn.href;
      } else if (e.key === "ArrowRight" && nextBtn) {
        window.location.href = nextBtn.href;
      }
    }
  });
}

/** Makes the sidebar collapsable on mobile. */
function setUpCollapsingSidebar() {
  /** @type {HTMLElement | null} */
  const nav = document.querySelector("nav.folding");
  if (!nav) return;

  nav.classList.add("mobile-hidden");
  nav.style.opacity = "1";

  const handleWindowClick = (e) => {
    if (!nav.classList.contains("mobile-hidden") && !nav.contains(e.target)) {
      hide();
      e.preventDefault();
    }
  };

  const finallyHide = () => {
    nav.classList.add("mobile-hidden");
    nav.style.transform = "translateX(0)";
  };

  const hide = () => {
    window.removeEventListener("click", handleWindowClick);
    if (hasTransitionEnd && !prefersReducedMotion) {
      nav.style.transform = "translateX(-100%)";
      nav.addEventListener("transitionend", finallyHide, { once: true });
    } else {
      finallyHide();
    }
  };

  const show = () => {
    nav.removeEventListener("transitionend", finallyHide);
    nav.style.transform = "translateX(-100%)";
    nav.classList.remove("mobile-hidden");
    requestAnimationFrame(() => {
      window.addEventListener("click", handleWindowClick);
      nav.style.transform = "translateX(0)";
    });
  };

  document.querySelector("button.hamburger")?.addEventListener("click", show);
  nav.querySelector("button.close")?.addEventListener("click", hide);
}

/**
 * Sets up the navigation bar on the left of the docs, enabling automatic
 * expansion/collapsing.
 */
function setUpFoldingNav() {
  document.querySelectorAll("nav.folding > ul").forEach(setUpAccordion);
}

/**
 * Sets up an expanding/collapsing accordion.
 *
 * @param target {HTMLUListElement}
 * @returns `true` if the accordion contains the current element
 */
function setUpAccordion(target) {
  const children = Array.from(target.children);
  const hasCurrent = children.some((child) =>
    child.querySelector("li > a[aria-current]"),
  );

  // All extensible elements, their button, their child and their expanded
  // height.
  let animated = false;
  for (const target of children) {
    if (target.tagName !== "LI") continue;

    const buttons = target.getElementsByTagName("button");
    const children = target.getElementsByTagName("ul");
    const anchors = target.getElementsByTagName("a");
    if (buttons.length === 0 || children.length === 0) {
      continue;
    }

    const button = buttons[0];
    const child = children[0];
    const childHasCurrent = setUpAccordion(child);
    const isCurrent = anchors[0]?.getAttribute("aria-current") === "page";

    setUpAccordionItem(
      target,
      button,
      child,
      childHasCurrent || isCurrent,
      false,
    );

    animated = true;
  }

  if (animated) {
    target.classList.add("animated");
  }

  return hasCurrent;
}

/**
 * Set up a single accordion item.
 *
 * @param target The complete folding container
 * @param button The button that toggles the folding
 * @param child The child that is folded
 * @param initiallyOpen Whether the child should be open initially
 */
function setUpAccordionItem(
  target,
  button,
  child,
  initiallyOpen,
  preventDefault = false,
) {
  const { toggle, expand, collapse } = toggleFactory(target, child);
  child.style.overflow = "visible hidden";

  const childHeight = Math.max(child.offsetHeight, 40 * child.children.length);
  if (!initiallyOpen) {
    collapse(childHeight, false);
  } else {
    expand(childHeight, false);
  }

  button.addEventListener("click", (e) => {
    if (preventDefault) {
      e.preventDefault();
    }

    toggle(childHeight);
  });
}

/**
 * Sets up nav folding for an element.
 *
 * @param target The complete folding container
 * @param child The child that is folded
 * @returns An object with three functions: To toggle, expand, and collapse
 */
function toggleFactory(target, child) {
  const { expand, handler } = expandFactory(target, child);
  const collapse = collapseFactory(target, child, handler);

  return {
    toggle: (childHeight) => {
      if (target.getAttribute("aria-expanded") === "true") {
        collapse(childHeight, true);
      } else {
        expand(childHeight, true);
      }
    },
    expand,
    collapse,
  };
}

/** Sets up an expansion handler for an element. */
function expandFactory(target, child) {
  const handler = () => {
    child.style.maxHeight = "none";
  };

  return {
    expand: (childHeight, animated) => {
      if (hasTransitionEnd && animated) {
        child.addEventListener("transitionend", handler, { once: true });
        child.style.maxHeight = `${childHeight}px`;
        child.style.pointerEvents = "auto";
        child.style.userSelect = "auto";
        child.style.opacity = "1";
      } else {
        handler();
      }
      target.setAttribute("aria-expanded", "true");
    },
    handler,
  };
}

/** Sets up a collapsing handler for an element. */
function collapseFactory(target, child, handler) {
  return (childHeight, animated) => {
    child.removeEventListener("transitionend", handler);
    if (animated) {
      child.style.maxHeight = `${childHeight}px`;
    }
    window.requestAnimationFrame(() => {
      child.style.pointerEvents = "none";
      child.style.userSelect = "none";
      child.style.maxHeight = "0px";
      child.style.opacity = "0";
    });
    target.setAttribute("aria-expanded", "false");
  };
}

/**
 * The on-page animation adds the "aria-current" attribute to the topmost
 * element in the viewport, turning it slightly blue in the "on this page"
 * outline.
 */
function setUpOnThisPage() {
  /**
   * @typedef Entry
   * @type {object}
   * @property item {HTMLLIElement}
   * @property assoc {HTMLElement}
   * @property anchor {HTMLAnchorElement}
   */

  /** @type {Entry[]} */
  const entries = [];

  /** @type {NodeListOf<HTMLLIElement>} */
  const items = document.querySelectorAll("#page-overview > ul li");
  for (const item of items) {
    const anchors = item.getElementsByTagName("a");
    if (anchors.length > 0) {
      const anchor = anchors[0];
      const url = new URL(anchor.href);
      const assocId = url.hash.replace(/^#/, "");
      const assoc = document.getElementById(assocId);
      if (assoc) {
        entries.push({ item: item, assoc, anchor });
      }
    }
  }

  const update = () => {
    const current = entries.find((item) => isElementInViewport(item.assoc));
    if (current) {
      for (const entry of entries) {
        entry.item.removeAttribute("aria-current");
      }
      current.item.setAttribute("aria-current", "true");
    }
  };

  update();
  window.addEventListener("scroll", update);
  window.addEventListener("resize", update);
}

/** Makes all tooltips on the page interactive. */
function setUpTooltips() {
  document.querySelectorAll("div.tooltip-context").forEach(setUpTooltip);
}

/** Makes a tooltip div interactive. */
function setUpTooltip(target) {
  const button = target.querySelector("svg");
  const tooltip = target.querySelector("div[role='tooltip']");
  if (!button || !tooltip) return;

  const title = button.getElementsByTagName("title")[0];
  const titleText = title.textContent;
  const tooltipWidth = 256;

  let lastShow = 0;
  let isOpen = false;

  const show = () => {
    const windowWidth = window.innerWidth;
    const rect = button.getBoundingClientRect();

    // If the tooltip would be outside the window, move it to the left
    if (rect.left + tooltipWidth / 2 > windowWidth) {
      tooltip.style.left = `${windowWidth - tooltipWidth - rect.left - 32}px`;
      tooltip.classList.add("mobile");
    } else if (rect.left - tooltipWidth / 2 < 0) {
      tooltip.style.left = `${-rect.left + 32}px`;
      tooltip.classList.add("mobile");
    } else {
      tooltip.style.left = "-120px";
      tooltip.classList.remove("mobile");
    }

    isOpen = true;
    title.innerHTML = "";
    tooltip.style.display = "block";
    window.requestAnimationFrame(() => {
      tooltip.style.opacity = "1";
      tooltip.style.pointerEvents = "auto";
      tooltip.removeAttribute("aria-hidden");
    });
    lastShow = Date.now();
  };

  const hide = () => {
    if (title) title.innerHTML = titleText ?? "";
    isOpen = false;
    const handler = () => {
      tooltip.style.display = "none";
      tooltip.style.pointerEvents = "none";
      tooltip.setAttribute("aria-hidden", "true");
    };
    if (hasTransitionEnd && !prefersReducedMotion) {
      tooltip.addEventListener("transitionend", handler, { once: true });
    } else {
      handler();
    }
    window.requestAnimationFrame(() => {
      if (hasTransitionEnd && !prefersReducedMotion) {
        tooltip.style.opacity = "0";
      }
    });
  };

  button.addEventListener("click", () => {
    if (!isHover) {
      if (isOpen && Date.now() - lastShow > 100) {
        hide();
      } else {
        show();
      }
    }
  });
  button.addEventListener("mouseenter", (e) => {
    e.preventDefault();
    show();
  });
  button.addEventListener("mousemove", (e) => {
    if (isOpen) {
      e.preventDefault();
    }
  });
  button.addEventListener("focus", show);
  button.addEventListener("blur", hide);
  button.addEventListener("mouseleave", hide);
  window.addEventListener("click", (e) => {
    if (isOpen && !target.contains(e.target)) {
      hide();
    }
  });
  window.addEventListener("mouseleave", hide);
  window.addEventListener("keydown", (e) => {
    if (isOpen && e.key === "Escape") {
      hide();
    }
  });
}

/**
 * Sets up how the examples with previews are split.
 *
 * Depending on the width of each example, sets a class that displays it
 * side-by-side vs top/bottom.
 */
function setUpPreviewSplits() {
  for (const example of document.querySelectorAll(".previewed-code > pre")) {
    if (example.clientWidth < example.scrollWidth) {
      example.classList.add("big");
    }
  }
}

/**
 * Sets up the overlays/flyouts that can be revealed by clicking on a symbol box
 * in a symbol lists.
 */
function setUpSymbolFlyouts() {
  for (const symbolGrid of document.querySelectorAll("main > .symbol-grid")) {
    setUpSymbolFlyout(symbolGrid);
  }
}

/** Sets up the overlay/flyout for a single symbol list. */
function setUpSymbolFlyout(symbolGrid) {
  const flyoutTemplate = /** @type {HTMLTemplateElement} */ (
    document.getElementById("flyout-template")
  );
  const flyoutRowTemplate = /** @type {HTMLTemplateElement} */ (
    document.getElementById("flyout-sym-row")
  );
  if (!symbolGrid || !flyoutTemplate || !flyoutRowTemplate) {
    return;
  }

  if (!("content" in document.createElement("template"))) {
    console.warn("Browser does not support template elements");
    return;
  }

  const flyout = /** @type {HTMLDivElement} */ (
    flyoutTemplate.content.firstElementChild.cloneNode(true)
  );

  flyout.style.display = "none";
  symbolGrid.appendChild(flyout);

  /** @type {HTMLElement | null} */
  const foButton = flyout.querySelector(".info button");
  /** @type {HTMLElement | null} */
  const foSymbol = flyout.querySelector(".info button .sym");
  /** @type {HTMLElement | null} */
  const foDeprecation = flyout.querySelector(".info .sym-deprecation");
  /** @type {HTMLElement | null} */
  const foDeprecationText = flyout.querySelector(
    ".info .sym-deprecation .text",
  );
  /** @type {HTMLElement | null} */
  const foName = flyout.querySelector(".sym-name code");
  /** @type {HTMLElement | null} */
  const foUnicName = flyout.querySelector(".info .unic-name");
  /** @type {HTMLElement | null} */
  const foLaTeXName = flyout.querySelector(".info .latex-name");
  /** @type {HTMLElement | null} */
  const foLaTeXNameCode = flyout.querySelector(".info .latex-name code");
  /** @type {HTMLElement | null} */
  const foMathClass = flyout.querySelector(".info .math-class");
  /** @type {HTMLElement | null} */
  const foMathClassSpan = flyout.querySelector(".info .math-class .value");
  /** @type {HTMLElement | null} */
  const foCodepoint = flyout.querySelector(".info .codepoint .value");
  /** @type {HTMLElement | null} */
  const foAccent = flyout.querySelector(".info .accent");
  /** @type {HTMLImageElement | null} */
  const foAccentIcon = flyout.querySelector(".info .accent img");
  /** @type {HTMLElement | null} */
  const foVariantsBox = flyout.querySelector(".variants-box");
  /** @type {HTMLElement | null} */
  const foAlternates = flyout.querySelector(".variants-box .symbol-grid");
  /** @type {HTMLElement | null} */
  const foShorthand = flyout.querySelector(".shorthand");
  /** @type {HTMLElement | null} */
  const foShorthandRemark = foShorthand.querySelector(".remark");
  /** @type {HTMLElement | null} */
  const foShorthandCode = flyout.querySelector(".shorthand code");
  /** @type {HTMLElement | null} */
  const foCopySymNameBtn = flyout.querySelector(".sym-name .copy");
  /** @type {HTMLElement | null} */
  const foCopyShorthandBtn = flyout.querySelector(".shorthand .copy");
  /** @type {HTMLElement | null} */
  const foCopyEscapeBtn = flyout.querySelector(".codepoint .copy");

  const listeners = [];

  let flyoutOpenId = undefined;
  const closeFlyout = () => {
    const btn = flyoutOpenId ? document.getElementById(flyoutOpenId) : null;
    if (btn) {
      btn.ariaHasPopup = "false";
    }
    flyoutOpenId = undefined;
    flyout.style.display = "none";
    window.removeEventListener("click", windowClickHandler);
    for (const { target, listener } of listeners) {
      target.removeEventListener("click", listener);
    }
  };

  const windowClickHandler = (e) => {
    if (
      flyoutOpenId !== undefined &&
      e.target &&
      "id" in e.target &&
      flyoutOpenId !== e.target?.id &&
      !flyout.contains(e.target)
    ) {
      closeFlyout();
      e.preventDefault();
    }
  };

  /** @param item List item with the data- attributes configure on the HTML. */
  const populateFlyout = (item) => {
    item.ariaHasPopup = "true";
    flyoutOpenId = item.id;
    flyout.style.display = "block";
    const name = item.id.replace(/^symbol-/, "");
    const deprecation = item.dataset.deprecation;
    const unicName = item.dataset.unicName;
    const latexName = item.dataset.latexName;
    const codepoint = item.dataset.value?.charCodeAt(0) ?? 0;
    const accent = item.dataset.accent != null;
    const alternates = item.dataset.alternates
      ? item.dataset.alternates.split(" ")
      : [];
    const mathShorthand = item.dataset.mathShorthand;
    const markupShorthand = item.dataset.markupShorthand;
    const shorthand = markupShorthand ?? mathShorthand;
    const mathClass = item.dataset.mathClass;
    const actualChar = item.dataset.value ?? "";
    const override = item.dataset.override;
    copyText(actualChar);

    let codepointText = codepoint.toString(16).toUpperCase();
    if (codepointText.length < 4) {
      codepointText = "0".repeat(4 - codepointText.length) + codepointText;
    }

    flyout.classList.toggle("override", override != null);
    foSymbol.textContent = override ?? actualChar;

    foDeprecation.style.display = deprecation ? "flex" : "none";
    if (deprecation) {
      foDeprecationText.textContent = deprecation.replaceAll("`", "");
    }

    foName.textContent = name;
    foUnicName.textContent = unicName ?? "";
    foCodepoint.textContent = codepointText;
    foAccent.style.display = accent ? "block" : "none";
    foAccentIcon.src = accent ? checkIconSrc : closeIconSrc;
    foAccentIcon.setAttribute("alt", accent ? "Yes" : "No");
    foShorthand.style.display =
      shorthand && shorthand.length > 0 ? "block" : "none";
    foShorthandCode.textContent = shorthand ?? "";

    if (latexName) {
      foLaTeXName.style.display = "block";
      foLaTeXNameCode.textContent = latexName;
    } else {
      foLaTeXName.style.display = "none";
    }

    if (mathClass) {
      foMathClass.style.display = "block";
      foMathClassSpan.textContent = mathClass;
    } else {
      foMathClass.style.display = "none";
    }

    const nameListener = () => copyText(name);
    foCopySymNameBtn.addEventListener("click", nameListener);
    listeners.push({ target: foCopySymNameBtn, listener: nameListener });

    if (shorthand && shorthand.length > 0) {
      const shorthandListener = () => copyText(shorthand);
      foCopyShorthandBtn.addEventListener("click", shorthandListener);
      listeners.push({
        target: foCopyShorthandBtn,
        listener: shorthandListener,
      });

      if (foShorthandRemark) {
        if (mathShorthand && !markupShorthand) {
          foShorthandRemark.textContent = "(only in math)";
          foShorthandRemark.style.display = "inline";
        } else if (!mathShorthand && markupShorthand) {
          foShorthandRemark.textContent = "(only in markup)";
          foShorthandRemark.style.display = "inline";
        } else {
          foShorthandRemark.style.display = "none";
        }
      }
    }

    const codepointListener = () => {
      copyText("\\u{" + codepointText + "}");
    };

    foCopyEscapeBtn.addEventListener("click", codepointListener);
    listeners.push({ target: foCopyEscapeBtn, listener: codepointListener });

    if (foVariantsBox !== null && foAlternates !== null) {
      const alternateElems = alternates
        .map((alt) => {
          const row = /** @type {HTMLLIElement} */ (
            flyoutRowTemplate.content.firstElementChild.cloneNode(true)
          );
          const altSymbol = row.querySelector(".sym");
          const altBtn = row.querySelector("button");
          const item = document.getElementById(`symbol-${alt}`);
          const altOverride = item?.dataset.override;

          if (item) {
            altBtn.classList.toggle("override", altOverride != null);
            altSymbol.textContent = altOverride ?? item.dataset.value ?? "";
          } else {
            return null;
          }

          altBtn.addEventListener("click", (e) => {
            populateFlyout(item);
            item.scrollIntoView();
            e.preventDefault();
          });

          return row;
        })
        .filter((r) => r != null);

      foVariantsBox.style.display =
        alternateElems.length > 0 ? "block" : "none";
      foAlternates.replaceChildren(...alternateElems);
    }

    // Position the flyout
    let left = item.offsetLeft - 12;
    const top = item.offsetTop - 12;

    if (item.getBoundingClientRect().left + 408 > window.innerWidth) {
      left = Math.max(
        8,
        window.innerWidth - symbolGrid.getBoundingClientRect().left - 424,
      );
    }

    flyout.style.left = `${left}px`;
    flyout.style.top = `${top}px`;
  };

  foButton?.addEventListener("click", closeFlyout);

  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && flyoutOpenId !== undefined) {
      closeFlyout();
      e.preventDefault();
    }
  });

  for (const item of symbolGrid.children) {
    if (item.tagName !== "LI") continue;
    item.addEventListener("click", (e) => {
      populateFlyout(item);
      e.preventDefault();
    });
  }

  window.requestAnimationFrame(() => {
    window.removeEventListener("click", windowClickHandler);
    window.addEventListener("click", windowClickHandler, { capture: true });
  });
}

/**
 * @typedef SearchIndex
 * @type {object}
 * @property items {IndexItem[]}
 * @property words {string[]}
 * @property hits {number[][]}
 */

/**
 * @typedef IndexItem
 * @type {object}
 * @property kind {string}
 * @property title {string}
 * @property route {string}
 * @property keywords {string[]}
 */

/** Sets up the docs-wide search. */
async function setUpGlobalSearch() {
  const textBox = /** @type {HTMLInputElement} */ (
    document.getElementById("docs-search")
  );
  const resultList = document.getElementById("search-results");
  if (!textBox || !resultList) return;

  const index = await fetchSearchIndex();
  if (!index) return;

  const search = () => {
    const query = textBox.value;
    const matches = searchGlobally(index, query);

    // Generate a `li > a` for each match.
    const items = matches.map((hit) => {
      const item = index.items[hit];
      let url = item.route;
      if (item.kind == "Symbols") {
        url += "?query=" + query;
      }
      const li = document.createElement("li");
      const a = document.createElement("a");
      const span = document.createElement("span");
      a.href = url;
      a.textContent = item.title;
      span.classList.add("type");
      span.textContent = item.kind;
      a.appendChild(span);
      li.appendChild(a);
      return li;
    });

    resultList.replaceChildren(...items);
    resultList.classList.toggle("hidden", matches.length === 0);
  };

  textBox.addEventListener("keyup", search);
  document.addEventListener("keyup", (event) => {
    if (event.key && event.key.toLowerCase() == "s") {
      event.stopPropagation();
      textBox.focus();
    }
  });

  if (textBox.value != "") {
    search();
  }
}

/**
 * Returns matches for docs-wide search.
 *
 * @param index {SearchIndex} The `search.json` index data
 * @param query {string} The search term
 * @returns {number[]} The indices of the matches in `index.items`, in ranked
 *   order.
 */
function searchGlobally(index, query) {
  const matches = [];
  const scores = [];

  function registerHit(hit) {
    // This is fast because the match list never exceeds a length of ten.
    if (matches.includes(hit)) return;

    const item = index.items[hit];
    scores[hit] = scoreItem(item, query);

    // We keep the match list small...
    if (matches.length < 10) {
      matches.push(hit);
      return;
    }

    // ...if it gets too long, we replace worse matches.
    let worst = 0;
    for (let i = 0; i < matches.length; i++) {
      if (scores[matches[i]] < scores[matches[worst]]) {
        worst = i;
      }
    }

    if (scores[hit] > scores[matches[worst]]) {
      matches[worst] = hit;
      return;
    }
  }

  // Each word is searches for separately.
  const queries = query
    .toLowerCase()
    .split(/[!-/:-@[-`{-~\s]/g)
    .filter((word) => word.length > 0);

  // Finds all words in `index.words` that start with the subquery and registers
  // matches for them.
  for (const subquery of queries) {
    // Binary search for any index word that starts with the subquery.
    let start = 0;
    let end = index.words.length - 1;
    while (start < end) {
      let mid = Math.floor((start + end) / 2);
      if (index.words[mid].startsWith(subquery)) {
        start = mid;
        break;
      } else if (subquery < index.words[mid]) {
        end = mid - 1;
      } else {
        start = mid + 1;
      }
    }

    // Go back to find the first word that starts with the subquery.
    let i = start;
    while (i > 0 && index.words[i].startsWith(subquery)) i--;
    i++;

    // Walk through all words that start with the subquery.
    while (i < index.words.length && index.words[i].startsWith(subquery)) {
      for (const hit of index.hits[i]) {
        registerHit(hit);
      }
      i++;
    }
  }

  // Rank by score.
  matches.sort((a, b) => scores[b] - scores[a]);

  return matches;
}

/** Tries to fetch the search index. */
async function fetchSearchIndex() {
  const abortController = new AbortController();
  const timeout = setTimeout(() => abortController.abort(), 10000);
  const index = await fetch(searchIndexSrc, {
    signal: abortController.signal,
  })
    .then((r) => (r.ok ? r.json() : null))
    .catch((r) => {
      console.warn("Failed to fetch search index", r);
      return null;
    });

  clearTimeout(timeout);
  return index;
}

/**
 * Computes a ranking score for an index item, given a query. Search hits are
 * sorted by score.
 *
 * @param item {IndexItem}
 * @param query {string}
 */
function scoreItem(item, query) {
  // Different kinds of index item have different priority in the ranking.
  const factors = {
    Type: 1.0,
    Function: 0.9,
    Parameter: 0.8,
    Method: 0.7,
    Symbols: 0.6,
    Chapter: 0.5,
    Category: 0.5,
  };
  const f = factors[item.kind.split(" ")[0]] ?? 0.4;
  return (
    f *
    Math.max(
      scoreText(item.title, query),
      ...(item.keywords || []).map((keyword) => scoreText(keyword, query)),
    )
  );
}

/**
 * Computes a similarly score for a potential hit and a search query. A higher
 * score means more similarity.
 *
 * This function is not typo aware.
 *
 * @param text {string}
 * @param query {string}
 */
function scoreText(text, query) {
  const textSimple = simplifyText(text);
  const querySimple = simplifyText(query);
  // The query should be contained in the text, otherwise we won't even find it.
  // Still, we give a score of zero otherwise.
  //
  // All else being equal, we prefer shorter matches. E.g., for the query
  // "Numb", we'd prefer "Numbering" over "Numbered List".
  return textSimple.includes(querySimple) ? 100 - text.length : 0;
}

/** Removes punctuation and symbols from text. */
function simplifyText(text) {
  return text.toLowerCase().replaceAll(/[!-/:-@[-`{-~\s]/g, "");
}

/** Makes the search box on the same pages active. */
function setUpSymbolSearch() {
  const symbolGrid = document.querySelector("main > .symbol-grid");
  const searchBox = /** @type {HTMLInputElement} */ (
    document.getElementById("symbol-search")
  );
  if (!symbolGrid || !searchBox) return;

  const symbols = /** @type {HTMLLIElement[]} */ (
    Array.from(symbolGrid.children).filter((c) => c.tagName === "LI")
  );

  const search = (event) => {
    const query = searchBox.value;
    const matches = new Set(
      query.length === 0 ? symbols : searchSymbols(symbols, query),
    );

    // Show matches and hide non-matches.
    for (const element of symbols) {
      if (matches.has(element)) {
        element.style.display = "block";
      } else {
        element.style.display = "none";
      }
    }

    // Currently, we don't reorder elements because the predictability of
    // keeping the alphabetic order is also nice.
    //
    // If we wanted to order elements by rank, we could do it like this:
    // ```
    // for (const element of matches) {
    //   element.parentNode.appendChild(element)
    // }
    // ```

    // Don't trigger global keybindings
    event?.stopPropagation();
  };

  searchBox.addEventListener("input", search);
  searchBox.addEventListener("keyup", search);

  const urlParams = new URLSearchParams(window.location.search);
  const query = urlParams.get("query");
  if (query) {
    searchBox.value = query;
  }

  if (searchBox.value != "") {
    search();
  }
}

/**
 * Find matches for a query in a symbol grid.
 *
 * @param symbols {HTMLElement[]} Elements with a relevant symbol dataset
 * @param query {string} The search term
 * @returns {HTMLElement[]} The matching elements from `symbols`, ranked order.
 */
function searchSymbols(symbols, query) {
  const codepoint = parseInt(
    query.replace(/^\\?[uU]\{?\+?/, "").replace(/\}$/, ""),
    16,
  );
  let char = codepoint ? String.fromCharCode(codepoint) : null;

  const list = [];
  for (const element of symbols) {
    let hit = false;
    for (const s of [
      element.id.replace(/^symbol-/, ""),
      element.dataset.unicName,
      element.dataset.latexName,
      element.dataset.value,
      element.dataset.shorthand,
      element.dataset.mathShorthand,
    ]) {
      if (
        typeof s === "string" &&
        s.toLowerCase().includes(query.toLowerCase())
      ) {
        hit = true;
      }
    }
    hit ||= char && element.dataset.value == char;
    if (hit) {
      list.push(element);
    }
  }

  return list;
}

/** Puts the given `text` into the clipboard. */
function copyText(text) {
  if ("clipboard" in navigator) {
    navigator.clipboard.writeText(text).catch((err) => {
      console.error("Failed to copy text to clipboard");
      console.error(err);
    });
    return;
  } else {
    const input = document.createElement("input");
    input.value = text;
    document.body.appendChild(input);
    input.select();
    document.execCommand("copy");
    document.body.removeChild(input);
  }
}

/** Returns whether an element is fully in the viewport. */
function isElementInViewport(el) {
  const rect = el.getBoundingClientRect();
  return (
    rect.top >= 0 &&
    rect.left >= 0 &&
    rect.bottom <=
      (window.innerHeight || document.documentElement.clientHeight) &&
    rect.right <= (window.innerWidth || document.documentElement.clientWidth)
  );
}

main();
