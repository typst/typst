const sidebarList = document.querySelector(".sidebar > .sidebar-list")
const globalSourceToggle = document.getElementById("global-view-test-sources")
/** @type {HTMLAnchorElement[]} */
const sidebarLinks = sidebarList.querySelectorAll("a")

/** @type {TestReportState[]} */
const testReports = []
/** @type {ImageDiffState} */
const imageDiffs = []

/**
 * @typedef TestReportState
 * @type {object}
 * @property report {HTMLDetailsElement}
 * @property reportToggle {HTMLButtonElement}
 * @property reportSourceToggle {HTMLButtonElement}
 * @property reportBody {HTMLDivElement}
 * @property reportSource {HTMLDivElement}
 * @property files {ReportFileState[]}
 */

/**
 * @typedef ReportFileState
 * @type {object}
 * @property report {TestReportState}
 * @property header {HTMLDivElement}
 * @property tab {HTMLInputElement}
 * @property tabpanel {HTMLElement}
 * @property diffs {FileDiffState[]}
 */

/**
 * @typedef FileDiffState
 * @type {object}
 * @property file {ReportFileState}
 * @property kind {DiffKind}
 * @property tab {HTMLInputElement?}
 * @property tabpanel {HTMLElement}
 */

/**
 * @typedef {"render" | "pdf" | "pdftags" | "svg" | "html"} TestOutput
 */

/**
 * @typedef {"visual" | "text"} DiffMode
 */

/**
 * @typedef {"text" | "image" | "html"} DiffKind
 */

/**
 * @typedef ImageDiffState
 * @type {object}
 * @property imageModes {HTMLInputElement[]}
 * @property imageAntialiasing {HTMLInputElement}
 * @property imageZoom {HTMLInputElement}
 * @property imageAlignXControl {HTMLElement}
 * @property imageAlignY {HTMLInputElement[]}
 * @property imageAlignX {HTMLInputElement[]}
 * @property imageBlendControl {HTMLElement}
 * @property imageBlend {HTMLInputElement}
 * @property canvases {ImageDiffCanvas[]}
 */


/**
 * @typedef ImageDiffCanvas
 * @type {object}
 * @property output {TestOutput}
 * @property visible {bool} Whether the canvas is visible.
 * @property imagesDecoded {bool} Whether the images have been decoded and their
 *                                natural dimensions are known.
 * @property state {CanvasDrawState} Whether the canvas should be (re-)drawn.
 * @property imageCanvas {HTMLCanvasElement}
 * @property images {HTMLImageElement[]}
 */

/**
 * @typedef {"side-by-side" | "swipe" | "blend" | "difference"} ImageViewMode
 */

/**
 * @typedef {"hidden" | "dirty" | "up-to-date"} CanvasDrawState
 */

let activeTestSources = 0

// Avoid implicit statefulness by the browser
globalSourceToggle.checked = false

for (const testReport of document.getElementsByClassName("test-report")) {
  const reportHeader = testReport.querySelector(".test-report-header")
  const reportToggle = reportHeader.querySelector(".test-report-toggle")
  const reportSourceToggle = reportHeader.querySelector(".test-report-source-toggle")
  const reportFileHeaders = reportHeader.querySelectorAll(".report-file-header");
  const reportFileTabGroup = reportHeader.querySelector(".report-file-tab-group");
  const reportFileTabs = reportFileTabGroup.querySelectorAll(".report-file-tab");
  const reportBody = testReport.querySelector(".test-report-body")
  const reportSource = reportBody.querySelector(".test-report-source")
  const reportFileTabpanels = reportBody.querySelectorAll(":scope > .report-file");

  /** @type {TestReportState} */
  const report = {
    report: testReport,
    reportBody,
    reportToggle,
    reportSourceToggle,
    reportSource,
    files: [],
  }
  testReports.push(report);

  reportToggle.addEventListener("click", () => {
    const expanded = !(reportToggle.ariaExpanded == "true");
    reportBody.hidden = !expanded;
    reportToggle.ariaExpanded = expanded;
  });

  reportSourceToggle.addEventListener("click", () => {
    const expanded = !(reportSourceToggle.ariaExpanded == "true");
    reportSource.hidden = !expanded;
    reportSourceToggle.ariaExpanded = expanded;

    if (expanded) {
      activeTestSources += 1;
    } else {
      activeTestSources -= 1;
    }

    globalSourceToggle.checked = activeTestSources > 0;
  });

  for (const button of reportHeader.querySelectorAll(".copy-button")) {
    button.addEventListener("click", () => {
      navigator.clipboard.writeText(button.dataset.filePath);
      button.classList.add("copied");
      setTimeout(() => button.classList.remove("copied"), 1)
    })
  }

  for (const [idx, tab] of reportFileTabs.entries()) {
    /** @type {ReportFileState} */
    const file = {
      report,
      tab,
      header: reportFileHeaders[idx],
      tabpanel: reportFileTabpanels[idx],
      diffs: [],
    };

    report.files.push(file)

    tab.addEventListener("change", (e) => {
      reportFileTabChanged(report, e.target.value, true)
    })
  }
  reportFileTabChanged(report, currentReportFileTab(report), false)


  /** @type {FileDiffState} */
  const reportImageDiffs = [];
  // Bind the height of the image diffs in a report.
  const resizeObserver = new ResizeObserver(entries => {
    const entry = entries[entries.length - 1];
    const newHeight = entry.target.style.height || "400px";
    for (const diff of reportImageDiffs) {
      if (diff.tabpanel != entry.target) {
        diff.tabpanel.style.height = newHeight;
      }
    }
  });

  for (const file of report.files) {
    // Not all file reports have multiple diffs. File diff tabs are only
    // generated when necessary.
    const tabWrapper = file.tab.parentElement.parentElement;
    let fileDiffTabs = []
    if (tabWrapper.classList.contains("report-file-tab-wrapper")) {
      fileDiffTabs = tabWrapper.querySelectorAll(".file-diff-tab");
    }

    const fileDiffTabpanels = file.tabpanel.querySelectorAll(":scope > .file-diff");
    for (const [idx, tabpanel] of fileDiffTabpanels.entries()) {
      const tab = fileDiffTabs[idx] || null;
      const kind = tabpanel.dataset.kind;
      /** @type {FileDiffState} */
      const diff = {
        file,
        kind,
        tab,
        tabpanel,
      };

      if (kind == "image") {
        resizeObserver.observe(diff.tabpanel);
        reportImageDiffs.push(diff);
      }

      file.diffs.push(diff);

      if (tab != null) {
        tab.addEventListener("change", (e) => {
          fileDiffTabChanged(file, e.target.value, true)
        })
      }
    }

    fileDiffTabChanged(file, currentFileDiffTab(file), false);
  }

  // There is one set of image controls for the image diffs of all output
  // formats inside a test report. This makes comparing the different formats
  // more convenient.
  const imageControlsTop = reportBody.querySelector(":scope > .image-controls.top")
  const imageControlsBottom = reportBody.querySelector(":scope > .image-controls.bottom")
  if (imageControlsTop != null && imageControlsBottom != null) {
    const imageModes = imageControlsTop.querySelectorAll("input.image-view-mode")
    const imageAntialiasing = imageControlsTop.querySelector("input.image-antialiasing")
    const imageZoom = imageControlsTop.querySelector("input.image-zoom")
    const imageZoomPlus = imageControlsTop.querySelector("button.image-zoom-plus")
    const imageZoomMinus = imageControlsTop.querySelector("button.image-zoom-minus")

    const imageAlignXControl = imageControlsBottom.querySelector(".image-align-x-control")
    const imageAlignX = imageAlignXControl.querySelectorAll(".image-align-x")
    const imageAlignYControl = imageControlsBottom.querySelector(".image-align-y-control")
    const imageAlignY = imageAlignYControl.querySelectorAll(".image-align-y")
    const imageBlendControl = imageControlsBottom.querySelector(".image-blend-control")
    const imageBlend = imageControlsBottom.querySelector("input.image-blend")

    /** @type {ImageDiffState} */
    const imageState = {
      imageModes,
      imageAntialiasing,
      imageZoom,
      imageAlignXControl,
      imageAlignX,
      imageAlignY,
      imageBlendControl,
      imageBlend,
      canvases: [],
    };
    imageDiffs.push(imageState);

    for (const imageMode of imageModes) {
      imageMode.addEventListener("change", (e) => {
        imageModeChanged(imageState, e.target.value);
      });
    }

    imageAntialiasing.addEventListener("change", () => imageDiffChanged(imageState));

    imageZoom.addEventListener("change", () => imageDiffChanged(imageState));
    imageZoom.addEventListener("input", () => imageDiffChanged(imageState));

    imageZoomMinus.addEventListener("click", () => {
      imageZoom.stepDown()
      imageDiffChanged(imageState)
    });
    imageZoomPlus.addEventListener("click", () => {
      imageZoom.stepUp()
      imageDiffChanged(imageState)
    });

    for (const align of imageAlignX) {
      align.addEventListener("change", () => imageDiffChanged(imageState));
    }
    for (const align of imageAlignY) {
      align.addEventListener("change", () => imageDiffChanged(imageState));
    }

    imageBlend.addEventListener("change", () => imageDiffChanged(imageState));
    imageBlend.addEventListener("input", () => imageDiffChanged(imageState));

    // Initially enable/disable the image controls.
    disableImageControls(imageState, currentImageMode(imageState));

    for (const imageDiff of reportBody.querySelectorAll(".file-diff.image")) {
      const imageCanvas = imageDiff.querySelector(".image-canvas")
      const images = imageCanvas.querySelectorAll("img")

      /** @type {ImageDiffCanvas} */
      const canvasState = {
        output: imageCanvas.dataset.output,
        visible: false,
        imagesDecoded: false,
        state: "hidden",
        imageCanvas,
        images,
      };

      // Issue a lazy canvas redaw when the images have been decoded.
      let numDecoded = 0;
      for (const img of images) {
        // Ignore invalid images.
        img.decode().catch(() => { }).then(() => {
          numDecoded += 1;
          if (numDecoded == images.length) {
            canvasState.imagesDecoded = true;
            redrawImageDiff(imageState, canvasState);
          }
        })
      }

      // Issue a lazy canvas redaw if the images become visible on screen.
      onViewportIntersectionChanged(imageDiff, (visible) => {
        canvasState.visible = visible;
        redrawImageDiff(imageState, canvasState);
      });

      imageState.canvases.push(canvasState);
    }
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

const diffModes = ["visual", "text"]
for (const mode of diffModes) {
  document.getElementById(`global-diff-mode-${mode}`)
    .addEventListener("click", () => {
      changeGlobalDiffMode(mode)
    })
}

globalSourceToggle
  .addEventListener("change", () => {
    // If all tests are hidden, display them. If one is shown, hide them.
    changeGlobalSourceVisibility(activeTestSources === 0)
  });

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
      for (const file of report.files) {
        if (outputs.includes(file.tab.value)) {
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
  for (const report of testReports) {
    let found = false
    for (const file of report.files) {
      if (file.tab.value == output) {
        file.tab.checked = true;
        found = true;
        break;
      }
    }
    if (found) {
      reportFileTabChanged(report, output, true)
    }
  }
}

/**
 * @param report {TestReportState}
 * @param output {TestOutput}
 * @param update_child {boolean}
 */
function reportFileTabChanged(report, output, update_child) {
  for (const file of report.files) {
    const selected = file.tab.value == output;
    file.tab.ariaSelected = selected;
    file.tabpanel.hidden = !selected;
    file.header.hidden = !selected;

    // Check which file is visible.
    if (update_child && selected) {
      fileDiffTabChanged(file, currentFileDiffTab(file), false);
    }
  }
}

/**
 * @param report {TestReportState}
 * @return {TestOutput}
 */
function currentReportFileTab(report) {
  for (const file of report.files) {
    if (file.tab.checked) {
      return file.tab.value
    }
  }
}

/**
 * @param diffMode {DiffMode}
 */
function changeGlobalDiffMode(diffMode) {
  for (const report of testReports) {
    for (const file of report.files) {
      let found = false
      for (const diff of file.diffs) {
        if (diff.tab?.value == diffMode) {
          diff.tab.checked = true;
          found = true;
          break;
        }
      }
      if (found) {
        fileDiffTabChanged(file, diffMode, false)
      }
    }
  }
}

/**
  * @param file {ReportFileState}
  * @param diffMode {DiffMode?}
  * @param update_parent {boolean}
  */
function fileDiffTabChanged(file, diffMode, update_parent) {
  let kind = null;
  if (diffMode == null) {
    console.assert(file.diffs.length == 1, "expected exactly one report file diff");

    kind = file.diffs[0].kind;
  } else {
    for (const diff of file.diffs) {
      const selected = diff.tab.value == diffMode;
      diff.tab.ariaSelected = selected;
      diff.tabpanel.hidden = !selected;

      if (selected) kind = diff.kind;
    }
  }

  // When the button of a nested tab is pressed, also update the parent tab.
  if (update_parent) {
    file.tab.checked = true;
    reportFileTabChanged(file.report, file.tab.value, false)
  }

  // When this tab is visible update the test report diff kind.
  if (file.tab.checked) {
    file.report.reportBody.classList.toggle("image", kind == "image")
  }
}

/**
  * @param file {ReportFileState}
  * @return {DiffMode?}
  */
function currentFileDiffTab(file) {
  for (const diff of file.diffs) {
    if (diff.tab?.checked) {
      return diff.tab.value
    }
  }

  // This only happens if there is only one file diff.
  return null;
}

/**
 * @param visible {boolean}
 */
function changeGlobalSourceVisibility(visible) {
  for (const report of testReports) {
    report.reportSource.hidden = !visible;
    report.reportSourceToggle.ariaExpanded = visible;
  }

  activeTestSources = visible ? testReports.length : 0;
}

/**
 * @param element {HTMLElement}
 * @param callback {(visible: bool) => void}
 */
function onViewportIntersectionChanged(element, callback) {
  const observer = new IntersectionObserver((entries, _observer) => {
    for (const entry of entries) {
      callback(entry.intersectionRatio > 0);
    }
  });

  observer.observe(element);
}

let imageModes = ["side-by-side", "swipe", "blend", "difference"]
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
    case "swipe": {
      state.imageAlignXControl.disabled = false;
      state.imageBlendControl.disabled = false;
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
 * @property clip {Path2D?}
 * @property opacity {number}
 * @property border_color {string}
 */

/**
 * @param state {ImageDiffState}
 */
function imageDiffChanged(state) {
  for (const canvas of state.canvases) {
    // Only mark the canvas as dirty if it was drawn to.
    // If it was "hidden" it is still hidden.
    if (canvas.state == "up-to-date") {
      canvas.state = "dirty";
    }
    redrawImageDiff(state, canvas);
  }
}

/**
 * @param state {ImageDiffState}
 * @param canvas {ImageDiffCanvas}
 */
function redrawImageDiff(state, canvas) {
  // In large test reports drawing only the image diffs that are on screen is
  // necessary to create a somewhat usable experience. It also dramatically
  // reduces loading time.
  if (!(canvas.visible && canvas.state != "up-to-date" && canvas.imagesDecoded)) {
    if (canvas.state == "dirty") {
      // Hide the canvas to avoid showing outdated drawn diff, when the canvas
      // comes into view.
      canvas.imageCanvas.hidden = true;
      canvas.state = "hidden";
    }
    return;
  }

  canvas.imageCanvas.hidden = false;
  canvas.state = "up-to-date";

  // HACK: Scale factor of HTML pt (`1/72 inch`) to px (`1/96 inch`).
  // Since PNG images are rendered with 1 px/pt and PDFs converted
  // to SVGs don't currently specify a unit thus default to px.
  const factor = (canvas.output == "svg") ? (72.0 / 96.0) : 1.0;
  const scale = factor * state.imageZoom.value
  const antialiased = state.imageAntialiasing.checked;
  const mode = currentImageMode(state)
  const alignX = currentImageAlignX(state);
  const alignY = currentImageAlignY(state);
  const blend = Number(state.imageBlend.value);

  /** @type {ImageParams} */
  const a = {
    x: 0,
    y: 0,
    w: scale * canvas.images[0].naturalWidth,
    h: scale * canvas.images[0].naturalHeight,
    opacity: 1,
    border_color: "#FF0000",
  };
  /** @type {ImageParams} */
  const b = {
    x: 0,
    y: 0,
    w: scale * canvas.images[1].naturalWidth,
    h: scale * canvas.images[1].naturalHeight,
    opacity: 1,
    border_color: "#00e030",
  };

  const maxWidth = Math.max(a.w, b.w);
  const maxHeight = Math.max(a.h, b.h);

  const sideBySideGap = 2;

  const swipeMargin = { x: 2, y: scale * 8 };
  const swipeDividerWidth = 1;

  // Logical size of the canvas without the margin.
  let canvasSize = { w: maxWidth, h: maxHeight };
  let canvasMargin = { x: 2, y: 2 };
  let compositeMode;
  let swipeDividerPos = null;
  switch (mode) {
    case "side-by-side": {
      compositeMode = "source-over";

      canvasSize = { w: 2 * maxWidth + sideBySideGap, h: maxHeight };

      // Center align images.
      a.x = maxWidth - a.w;
      b.x = maxWidth + sideBySideGap;

      a.y = verticalAlignImage(a, canvasSize, alignY);
      b.y = verticalAlignImage(b, canvasSize, alignY);

      break;
    }
    case "swipe": {
      compositeMode = "source-over";

      canvasMargin = swipeMargin;

      a.x = horizontalAlignImage(a, canvasSize, alignX)
      b.x = horizontalAlignImage(b, canvasSize, alignX)
      a.y = verticalAlignImage(a, canvasSize, alignY)
      b.y = verticalAlignImage(b, canvasSize, alignY)

      swipeDividerPos = {
        x: Math.round(blend * canvasSize.w),
        y: -swipeMargin.y,
      };

      // Use clip paths instead of the `drawImage` source paramters to avoid
      // wobble of the images when moving the slider.
      a.clip = new Path2D();
      a.clip.rect(0, 0, swipeDividerPos.x, canvasSize.h);

      b.clip = new Path2D();
      b.clip.rect(swipeDividerPos.x, 0, canvasSize.w - swipeDividerPos.x, canvasSize.h);

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

  // Computation is done, do the actual drawing.
  canvas.imageCanvas.width = canvasSize.w + 2 * canvasMargin.x;
  canvas.imageCanvas.height = canvasSize.h + 2 * canvasMargin.y;

  const ctx = canvas.imageCanvas.getContext("2d")
  ctx.clearRect(0, 0, canvas.imageCanvas.width, canvas.imageCanvas.height);

  ctx.imageSmoothingEnabled = antialiased;
  ctx.globalCompositeOperation = compositeMode;
  ctx.translate(canvasMargin.x, canvasMargin.y);

  drawImage(ctx, canvas.images[0], a);
  drawImage(ctx, canvas.images[1], b);

  // Divider.
  if (swipeDividerPos != null) {
    ctx.lineWidth = swipeDividerWidth;
    ctx.strokeStyle = "#007BFF";
    const divider = new Path2D();
    divider.moveTo(swipeDividerPos.x, swipeDividerPos.y);
    divider.lineTo(swipeDividerPos.x, canvas.imageCanvas.height);
    ctx.stroke(divider);
  }
}

/**
 * @param ctx {CanvasRenderingContext2D}
 * @param img {HTMLImageElement}
 * @param p {ImageParams}
 */
function drawImage(ctx, img, p) {
  ctx.save();
  ctx.globalAlpha = p.opacity;

  // Draw image.
  if (p.clip != null) ctx.clip(p.clip);
  ctx.drawImage(img, p.x, p.y, p.w, p.h);

  // Draw outline.
  ctx.lineWidth = 2;
  ctx.strokeStyle = p.border_color;
  ctx.strokeRect(p.x, p.y, p.w, p.h);

  ctx.restore();
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
 * @returns {ImageViewMode}
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
