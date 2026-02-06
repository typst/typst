const sidebarList = document.querySelector(".sidebar > .sidebar-list")
/** @type {HTMLAnchorElement[]} */
const sidebarLinks = sidebarList.querySelectorAll("a")

/** @type {TestReportState[]} */
const testReports = []
/** @type {ReportFileState[]} */
const reportFiles = []

/**
 * @typedef TestReportState
 * @type {object}
 * @property report {HTMLDetailsElement}
 * @property reportToggle {HTMLButtonElement}
 * @property reportFileHeaders {NodeListOf<HTMLDivElement>}
 * @property reportFileTabs {NodeListOf<HTMLInputElement>}
 * @property reportBody HTMLDivElement
 * @property reportFileTabpanels {NodeListOf<HTMLElement>}
 */

/**
 * @typedef {"render" | "pdf" | "pdftags" | "svg" | "html"} TestOutput
 */

/**
 * @typedef ReportFileState
 * @type {object}
 * @property fileDiffTabs {NodeListOf<HTMLInputElement>}
 * @property fileDiffTabpanels {NodeListOf<HTMLElement>}
 */

/**
 * @typedef {"image" | "text"} DiffKind
 */

for (const report of document.getElementsByClassName("test-report")) {
  const reportHeader = report.querySelector(".test-report-header")
  const reportToggle = reportHeader.querySelector(".test-report-toggle")
  const reportFileHeaders = reportHeader.querySelectorAll(".report-file-header");
  const reportFileTabGroup = reportHeader.querySelector(".report-file-tab-group");
  const reportFileTabs = reportFileTabGroup.querySelectorAll(".report-file-tab");
  const reportBody = report.querySelector(".test-report-body")
  const reportFileTabpanels = reportBody.querySelectorAll(":scope > .report-file");

  /** @type {TestReportState} */
  const state = {
    report,
    reportBody,
    reportFileHeaders,
    reportFileTabs,
    reportFileTabpanels,
    reportToggle,
  }
  testReports.push(state);

  reportToggle.addEventListener("click", () => {
    const expanded = !(reportToggle.ariaExpanded == "true");
    reportBody.hidden = !expanded;
    reportToggle.ariaExpanded = expanded;
  });

  for (const button of reportHeader.querySelectorAll(".copy-button")) {
    button.addEventListener("click", () => {
      navigator.clipboard.writeText(button.dataset.filePath);
      button.classList.add("copied");
      setTimeout(() => button.classList.remove("copied"), 1)
    })
  }

  for (const tab of reportFileTabs) {
    tab.addEventListener("change", (e) => {
      reportFileTabChanged(state, e.target.value)
    })
  }
  reportFileTabChanged(state, currentReportFileTab(state))

  for (const [idx, child] of reportFileTabGroup.childNodes.entries()) {
    const fileDiffTabGroup = child.querySelector(".file-diff-tab-group");

    // Not all file reports have multiple diffs. File diff tabs are only
    // generated when necessary.
    if (fileDiffTabGroup == null) {
      continue;
    }

    const fileDiffTabs = fileDiffTabGroup.querySelectorAll(".file-diff-tab");
    const fileDiffTabpanels = reportFileTabpanels[idx].querySelectorAll(":scope > .file-diff");

    /** @type {ReportFileState} */
    const state = {
      fileDiffTabs,
      fileDiffTabpanels,
    }
    reportFiles.push(state);

    for (const tab of fileDiffTabs) {
      tab.addEventListener("change", (e) => {
        fileDiffTabChanged(state, e.target.value)
      })
    }
    fileDiffTabChanged(state, currentFileDiffTab(state))
  }
}

let outputs = ["render", "pdf", "pdftags", "svg", "html"]
/** @type {HTMLInputElement[]} */
let filterDiffFormats = []
for (const output of outputs) {
  let filterFormat = document.getElementById(`filter-diff-format-${output}`);
  filterDiffFormats.push(filterFormat)
  filterFormat.addEventListener("change", () => {
    filterDiffs()
  });

  document.getElementById(`global-diff-format-${output}`)
    .addEventListener("click", () => {
      changeGlobalDiffFormat(output)
    })
}

/** @type {HTMLInputElement} */
let filterSearch = document.getElementById("filter-search");
filterSearch.addEventListener("change", () => {
  filterDiffs()
})

filterDiffs()

let diff_kinds = ["text", "image"]
for (const kind of diff_kinds) {
  document.getElementById(`global-diff-mode-${kind}`)
    .addEventListener("click", () => {
      changeGlobalDiffMode(kind)
    })
}

function filterDiffs() {
  let outputs = filterDiffFormats
    .filter((filterFormat) => !filterFormat.disabled && filterFormat.checked)
    .map((filterFormat) => filterFormat.value)
  let text = filterSearch.value

  for (const [i, report] of testReports.entries()) {
    // Ids are just the test names prefixed with `r-`.
    const name = report.report.id.substring(2);
    let filteredOut = false;

    // Filter search
    if (text.length > 0) {
      filteredOut = !name.includes(text);
    }

    // Filter format
    if (outputs.length > 0 && !filteredOut) {
      filteredOut = true;
      for (const tab of report.reportFileTabs) {
        if (outputs.includes(tab.value)) {
          filteredOut = false;
          break;
        }
      }
    }

    report.report.hidden = filteredOut;
    sidebarLinks[i].hidden = filteredOut;
  }
}

/**
 * @param output {TestOutput}
 */
function changeGlobalDiffFormat(output) {
  for (const state of testReports) {
    let found = false
    for (const tab of state.reportFileTabs) {
      if (tab.value == output) {
        tab.checked = true;
        found = true;
        break;
      }
    }
    if (found) {
      reportFileTabChanged(state, output)
    }
  }
}

/**
 * @param state {TestReportState}
 * @param output {TestOutput}
 */
function reportFileTabChanged(state, output) {
  for (const [idx, tab] of state.reportFileTabs.entries()) {
    const selected = tab.value == output;
    tab.ariaSelected = selected;
    state.reportFileTabpanels[idx].hidden = !selected;
    state.reportFileHeaders[idx].hidden = !selected;
  }
}

/**
 * @param state {TestReportState}
 * @return {TestOutput}
 */
function currentReportFileTab(state) {
  for (const tab of state.reportFileTabs) {
    if (tab.checked) {
      return tab.value
    }
  }
}

/**
 * @param diff_kind {DiffKind}
 */
function changeGlobalDiffMode(diff_kind) {
  for (const state of reportFiles) {
    let found = false
    for (const tab of state.fileDiffTabs) {
      if (tab.value == diff_kind) {
        tab.checked = true;
        found = true;
        break;
      }
    }
    if (found) {
      fileDiffTabChanged(state, diff_kind)
    }
  }
}

/**
  * @param state {ReportFileState}
  * @param diff_kind {DiffKind}
  */
function fileDiffTabChanged(state, diff_kind) {
  for (const [idx, tab] of state.fileDiffTabs.entries()) {
    const selected = tab.value == diff_kind;
    tab.ariaSelected = selected;
    state.fileDiffTabpanels[idx].hidden = !selected;
  }
}

/**
  * @param state {ReportFileState}
  * @return {DiffKind}
  */
function currentFileDiffTab(state) {
  for (const tab of state.fileDiffTabs) {
    if (tab.checked) {
      return tab.value
    }
  }
}

/** @type {ImageDiffState} */
const imageDiffs = []

/**
 * @typedef ImageDiffState
 * @type {object}
 * @property dirty {bool} Whether the canvas should be (re-)drawn.
 * @property visible {bool} Whether the canvas is visible.
 * @property imageCanvas {HTMLCanvasElement}
 * @property images {HTMLImageElement[]}
 * @property imageModes {HTMLInputElement[]}
 * @property imageAntialiasing {HTMLInputElement}
 * @property imageZoom {HTMLInputElement}
 * @property imageAlignXControl {HTMLElement}
 * @property imageAlignY {HTMLInputElement[]}
 * @property imageAlignX {HTMLInputElement[]}
 * @property imageBlendControl {HTMLElement}
 * @property imageBlend {HTMLInputElement}
 */

/**
 * @typedef {"side-by-side" | "blend" | "difference"} ImageViewMode
 */

for (const imageDiff of document.getElementsByClassName("image-diff")) {
  const imageWrapper = imageDiff.querySelector(".image-diff-wrapper")
  const imageCanvas = imageWrapper.querySelector(".image-canvas")
  const images = imageCanvas.querySelectorAll("img")

  const imageModes = imageDiff.querySelectorAll("input.image-view-mode")
  const imageAntialiasing = imageDiff.querySelector("input.image-antialiasing")
  const imageZoom = imageDiff.querySelector("input.image-zoom")
  const imageZoomPlus = imageDiff.querySelector("button.image-zoom-plus")
  const imageZoomMinus = imageDiff.querySelector("button.image-zoom-minus")
  const imageAlignXControl = imageDiff.querySelector(".image-align-x-control")
  const imageAlignX = imageAlignXControl.querySelectorAll(".image-align-x")
  const imageAlignYControl = imageDiff.querySelector(".image-align-y-control")
  const imageAlignY = imageAlignYControl.querySelectorAll(".image-align-y")
  const imageBlendControl = imageDiff.querySelector(".image-blend-control")
  const imageBlend = imageDiff.querySelector("input.image-blend")

  /** @type {ImageDiffState} */
  const state = {
    dirty: true,
    visible: false,
    imageCanvas,
    images,
    imageModes,
    imageAntialiasing,
    imageZoom,
    imageAlignXControl,
    imageAlignX,
    imageAlignY,
    imageBlendControl,
    imageBlend,
  }
  imageDiffs.push(state);

  for (const imageMode of imageModes) {
    imageMode.addEventListener("change", (e) => {
      imageModeChanged(state, e.target.value);
    });
  }

  imageAntialiasing.addEventListener("change", () => imageDiffChanged(state));

  imageZoom.addEventListener("change", () => imageDiffChanged(state));
  imageZoom.addEventListener("input", () => imageDiffChanged(state));

  imageZoomMinus.addEventListener("click", () => {
    imageZoom.stepDown()
    imageDiffChanged(state)
  });
  imageZoomPlus.addEventListener("click", () => {
    imageZoom.stepUp()
    imageDiffChanged(state)
  });

  for (const align of imageAlignX) {
    align.addEventListener("change", () => imageDiffChanged(state));
  }
  for (const align of imageAlignY) {
    align.addEventListener("change", () => imageDiffChanged(state));
  }

  imageBlend.addEventListener("change", () => imageDiffChanged(state));
  imageBlend.addEventListener("input", () => imageDiffChanged(state));

  // Initially enable/disable the image controls.
  disableImageControls(state, currentImageMode(state));

  // Issue a lazy canvas redaw if the images become visible on screen.
  onViewportIntersectionChanged(imageWrapper, (visible) => {
    state.visible = visible;
    redrawImageDiff(state);
  });
}

/**
 * @param element {HTMLElement}
 * @param callback {(visible: bool) => void}
 */
function onViewportIntersectionChanged(element, callback) {
  var observer = new IntersectionObserver((entries, _observer) => {
    for (const entry of entries) {
      callback(entry.intersectionRatio > 0);
    }
  });

  observer.observe(element);
}

let imageModes = ["side-by-side", "blend", "difference"]
for (const mode of imageModes) {
  document.getElementById(`global-image-view-mode-${mode}`)
    .addEventListener("click", () => changeGlobalImageMode(mode));
}

/**
 * @param mode {ImageViewMode}
 */
function changeGlobalImageMode(mode) {
  for (const state of imageDiffs) {
    for (const imageMode of state.imageModes) {
      if (imageMode.value == mode) {
        imageMode.checked = true;
      }
    }
    imageModeChanged(state, mode);
  }
}

/**
 * @param state {ImageDiffState}
 * @param mode {ImageViewMode}
 */
function imageModeChanged(state, mode) {
  disableImageControls(state, mode);
  imageDiffChanged(state);
}

/**
 * @param state {ImageDiffState}
 * @param mode {ImageViewMode}
 */
function disableImageControls(state, mode) {
  switch (mode) {
    case "side-by-side": {
      state.imageAlignXControl.disabled = true;
      state.imageBlendControl.disabled = true;
      break;
    }
    case "blend": {
      state.imageAlignXControl.disabled = false;
      state.imageBlendControl.disabled = false;
      break;
    }
    case "difference": {
      state.imageAlignXControl.disabled = false;
      state.imageBlendControl.disabled = false;
      break;
    }
    default: throw `unknown mode ${mode}`
  }
}

/**
 * @typedef ImageParams
 * @type {object}
 * @property x {number}
 * @property y {number}
 * @property w {number}
 * @property h {number}
 * @property opacity {number}
 */

/**
 * @param state {ImageDiffState}
 */
function imageDiffChanged(state) {
  state.dirty = true;
  redrawImageDiff(state);
}

/**
 * @param state {ImageDiffState}
 */
function redrawImageDiff(state) {
  // In large test reports drawing only the image diffs that are on screen is
  // necessary to create a somewhat usable experience. It also dramatically
  // reduces loading time.
  if (!state.dirty || !state.visible) return;

  state.dirty = false;

  const scale = state.imageZoom.value
  const antialiased = state.imageAntialiasing.checked;
  const mode = currentImageMode(state)
  const alignX = currentImageAlignX(state);
  const alignY = currentImageAlignY(state);
  const blend = state.imageBlend.value

  const a = {
    x: 0,
    y: 0,
    w: scale * state.images[0].naturalWidth,
    h: scale * state.images[0].naturalHeight,
    opacity: 1,
  };
  const b = {
    x: 0,
    y: 0,
    w: scale * state.images[1].naturalWidth,
    h: scale * state.images[1].naturalHeight,
    opacity: 1,
  };

  const maxWidth = Math.max(a.w, b.w);
  const maxHeight = Math.max(a.h, b.h);

  const sideBySideGap = 1;

  let canvasSize = { w: maxWidth, h: maxHeight };
  let compositeMode;
  switch (mode) {
    case "side-by-side": {
      compositeMode = "source-over";

      const maxWidth = Math.max(a.w, b.w);
      canvasSize = { w: 2 * maxWidth + sideBySideGap, h: Math.max(a.h, b.h) };

      // Center align images
      a.x = maxWidth - a.w;
      b.x = maxWidth + sideBySideGap;

      a.y = verticalAlignImage(a, canvasSize, alignY);
      b.y = verticalAlignImage(b, canvasSize, alignY);

      break;
    }
    case "blend": {
      // Additive mixing.
      compositeMode = "lighter";

      // Blend images.
      a.opacity = 1 - blend;
      b.opacity = blend;

      a.x = horizontalAlignImage(a, canvasSize, alignX);
      b.x = horizontalAlignImage(b, canvasSize, alignX);
      a.y = verticalAlignImage(a, canvasSize, alignY);
      b.y = verticalAlignImage(b, canvasSize, alignY);

      break;
    }
    case "difference": {
      compositeMode = "difference";

      // Fade out one of the images.
      if (blend < 0.4) {
        b.opacity = (blend / 0.4);
      } else if (blend > 0.6) {
        a.opacity = 1 - ((blend - 0.6) / 0.4);
      }

      a.x = horizontalAlignImage(a, canvasSize, alignX);
      b.x = horizontalAlignImage(b, canvasSize, alignX);
      a.y = verticalAlignImage(a, canvasSize, alignY);
      b.y = verticalAlignImage(b, canvasSize, alignY);

      break;
    }
    default: throw `unknown mode ${mode}`
  }

  state.imageCanvas.width = canvasSize.w;
  state.imageCanvas.height = canvasSize.h;

  const ctx = state.imageCanvas.getContext("2d")
  ctx.clearRect(0, 0, state.imageCanvas.width, state.imageCanvas.height);

  ctx.imageSmoothingEnabled = antialiased;
  ctx.globalCompositeOperation = compositeMode;

  ctx.globalAlpha = a.opacity;
  ctx.drawImage(state.images[0], a.x, a.y, a.w, a.h);

  ctx.globalAlpha = b.opacity;
  ctx.drawImage(state.images[1], b.x, b.y, b.w, b.h);
}

/**
 * @param img {ImageParams}
 * @param size {{w: number, h: number}}
 * @param align: {"left" | "center" | "right"}
 * @returns number
 */
function horizontalAlignImage(img, size, align) {
  switch (align) {
    case "left": return 0;
    case "center": return 0.5 * (size.w - img.w);
    case "right": return size.w - img.w;
    default: throw `unknown horizontal alignment ${align}`
  }
}

/**
 * @param img {ImageParams}
 * @param size {{w: number, h: number}}
 * @param align: { "top" | "center" | "bottom" }
 * @returns number
 */
function verticalAlignImage(img, size, align) {
  switch (align) {
    case "top": return 0;
    case "center": return 0.5 * (size.h - img.h);
    case "bottom": return size.h - img.h;
    default: throw `unknown vertical alignment ${align}`
  }
}

/**
 * @param state {ImageDiffState}
 */
function currentImageMode(state) {
  for (const imageMode of state.imageModes) {
    if (imageMode.checked) {
      return imageMode.value
    }
  }
}

/**
 * @param state {ImageDiffState}
 */
function currentImageAlignX(state) {
  for (const align of state.imageAlignX) {
    if (align.checked) {
      return align.value
    }
  }
}

/**
 * @param state {ImageDiffState}
 */
function currentImageAlignY(state) {
  for (const align of state.imageAlignY) {
    if (align.checked) {
      return align.value
    }
  }
}
