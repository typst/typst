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
 * @property imageSplits {HTMLDivElement[]}
 * @property images {HTMLImageElement[]}
 * @property imageModes {HTMLInputElement[]}
 * @property imageZoom {HTMLInputElement}
 * @property imageAlignXControl {HTMLInputElement[]}
 * @property imageBlendControl {HTMLFieldSetElement}
 * @property imageBlend {HTMLInputElement}
 */

/**
 * @typedef {"side-by-side" | "blend" | "difference"} ImageViewMode
 */

for (const imageDiff of document.getElementsByClassName("image-diff")) {
  const imageWrapper = imageDiff.querySelector(".image-diff-wrapper")
  const imageSplits = imageWrapper.querySelectorAll(".image-split")
  const images = imageWrapper.querySelectorAll("img")

  const imageModes = imageDiff.querySelectorAll("input.image-view-mode")
  const imageZoom = imageDiff.querySelector("input.image-zoom")
  const imageZoomPlus = imageDiff.querySelector("button.image-zoom-plus")
  const imageZoomMinus = imageDiff.querySelector("button.image-zoom-minus")
  const imageAlignXControl = imageDiff.querySelector(".image-align-x-control")
  const imageBlendControl = imageDiff.querySelector(".image-blend-control")
  const imageBlend = imageDiff.querySelector("input.image-blend")

  /** @type {ImageDiffState} */
  const state = {
    imageSplits,
    images,
    imageModes,
    imageZoom,
    imageAlignXControl,
    imageBlendControl,
    imageBlend,
  }
  imageDiffs.push(state)

  imageZoom.addEventListener("change", () => setImageZoom(state))
  imageZoom.addEventListener("input", () => setImageZoom(state))
  setImageZoom(state)

  imageZoomMinus.addEventListener("click", () => {
    imageZoom.stepDown()
    setImageZoom(state)
  })
  imageZoomPlus.addEventListener("click", () => {
    imageZoom.stepUp()
    setImageZoom(state)
  })

  imageBlend.addEventListener("change", () => setImageBlend(state))
  imageBlend.addEventListener("input", () => setImageBlend(state))

  for (const imageMode of imageModes) {
    imageMode.addEventListener("change", (e) => {
      imageModeChanged(state, e.target.value)
    })
  }

  const mode = currentImageMode(state);
  imageModeChanged(state, mode)
}

let imageModes = ["side-by-side", "blend", "difference"]
for (const mode of imageModes) {
  document.getElementById(`global-image-view-mode-${mode}`)
    .addEventListener("click", () => {
      changeGlobalImageMode(mode)
    })
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
    imageModeChanged(state, mode)
  }
}

/**
 * @param state {ImageDiffState}
 * @param mode {ImageViewMode}
 */
function imageModeChanged(state, mode) {
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
      state.imageBlendControl.disabled = true;
      break;
    }
    default: throw `unknown mode ${mode}`
  }

  setImageBlend(state)
}

/**
 * @param state {ImageDiffState}
 */
function setImageZoom(state) {
  const scale = state.imageZoom.value
  for (const image of state.imageSplits) {
    image.style.transform = `scale(${scale})`
  }
}

/**
 * @param state {ImageDiffState}
 */
function setImageBlend(state) {
  const mode = currentImageMode(state)
  const blend = state.imageBlend.value
  state.images[0].style.opacity = (mode == "blend") ? (1 - blend) : 1
  state.images[1].style.opacity = (mode == "blend") ? blend : 1
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
