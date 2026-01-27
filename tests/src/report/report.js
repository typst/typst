const imageDiffs = []

/**
  * @typedef TestReportState
* @type {object}
* @property reportFileTabs {NodeListOf<HTMLInputElement>}
  * @property reportFileTabpanels {NodeListOf<Element>}
  *
*/

/**
* @typedef {"render" | "pdf" | "pdftags" | "svg" | "html"}TestOutput
*/

/**
  * @typedef FileDiffState
* @type {object}
* @property fileDiffTabs {NodeListOf<HTMLInputElement>}
  * @property fileDiffTabpanels {NodeListOf<Element>}
  *
*/

/**
* @typedef {"image" | "text"} DiffKind
*/

for (const testReport of document.getElementsByClassName("test-report")) {
  const reportFileTabGroup = testReport.querySelector(".report-file-tab-group");
  const reportFileTabs = reportFileTabGroup.querySelectorAll(".report-file-tab");
  // TODO: More efficient query
  const reportFileTabpanels = testReport.querySelectorAll(".file-report");

  /** @type {TestReportState} */
  const state = {
    reportFileTabs,
    reportFileTabpanels,
  }

  for (const tab of reportFileTabs) {
    tab.addEventListener("change", (e) => {
      reportFileTabChanged(state, e.target.value)
    })
  }
  reportFileTabChanged(state, currentReportFileTab(state))


  for (const tabpanel of reportFileTabpanels) {
    const fileDiffTabGroup = tabpanel.querySelector(".file-diff-tab-group");

    // Not all file reports have multiple diffs. File diff tabs are only
    // generated when necessary.
    if (fileDiffTabGroup == null) {
      continue;
    }

    const fileDiffTabs = fileDiffTabGroup.querySelectorAll(".file-diff-tab");
    // TODO: More efficient query
    const fileDiffTabpanels = tabpanel.querySelectorAll(".file-diff");

    /** @type {FileDiffState} */
    const state = {
      fileDiffTabs,
      fileDiffTabpanels,
    }

    for (const tab of fileDiffTabs) {
      tab.addEventListener("change", (e) => {
        fileDiffTabChanged(state, e.target.value)
      })
    }
    fileDiffTabChanged(state, currentFileDiffTab(state))
  }
}

/**
  * @param state {TestReportState}
  * @param output {TestOutput}
  */
function reportFileTabChanged(state, output) {
  for (const [idx, tab] of state.reportFileTabs.entries()) {
    if (tab.value == output) {
      tab.ariaSelected = true;
      state.reportFileTabpanels[idx].style.display = "";
    } else {
      tab.ariaSelected = false;
      state.reportFileTabpanels[idx].style.display = "none";
    }
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
  * @param state {FileDiffState}
  * @param diff_kind {DiffKind}
  */
function fileDiffTabChanged(state, diff_kind) {
  for (const [idx, tab] of state.fileDiffTabs.entries()) {
    if (tab.value == diff_kind) {
      tab.ariaSelected = true;
      state.fileDiffTabpanels[idx].style.display = "";
    } else {
      tab.ariaSelected = false;
      state.fileDiffTabpanels[idx].style.display = "none";
    }
  }
}

/**
  * @param state {FileDiffState}
  * @return {DiffKind}
  */
function currentFileDiffTab(state) {
  for (const tab of state.fileDiffTabs) {
    if (tab.checked) {
      return tab.value
    }
  }
}

for (const imageDiff of document.getElementsByClassName("image-diff")) {
  const imageWrapper = imageDiff.querySelector(".image-diff-wrapper")
  const images = imageWrapper.querySelectorAll("img")

  const imageModes = imageDiff.querySelectorAll("input.image-view-mode")
  const imageZoom = imageDiff.querySelector("input.image-zoom")
  const imageZoomPlus = imageDiff.querySelector("button.image-zoom-plus")
  const imageZoomMinus = imageDiff.querySelector("button.image-zoom-minus")
  const imageAlignXControl = imageDiff.querySelector(".image-align-x-control")
  const imageBlendControl = imageDiff.querySelector(".image-blend-control")
  const imageBlend = imageDiff.querySelector("input.image-blend")

  const state = {
    imageWrapper,
    images,
    imageModes,
    imageZoom,
    imageAlignXControl,
    imageBlendControl,
    imageBlend,
  }
  imageDiffs.push(state)

  imageZoom.addEventListener("change", (e) => setImageZoom(state))
  imageZoom.addEventListener("input", (e) => setImageZoom(state))
  setImageZoom(state)

  imageZoomMinus.addEventListener("click", (e) => {
    imageZoom.stepDown()
    setImageZoom(state)
  })
  imageZoomPlus.addEventListener("click", (e) => {
    imageZoom.stepUp()
    setImageZoom(state)
  })

  imageBlend.addEventListener("change", (e) => setImageBlend(state))
  imageBlend.addEventListener("input", (e) => setImageBlend(state))

  for (const imageMode of imageModes) {
    imageMode.addEventListener("change", (e) => {
      imageModeChanged(state, e.target.value)
    })
  }

  const mode = currentImageMode(state);
  imageModeChanged(state, mode)
}

document.getElementById("global-image-view-mode-side-by-side").addEventListener("click", (e) => {
  changeGlobalImageMode("side-by-side")
})
document.getElementById("global-image-view-mode-blend").addEventListener("click", (e) => {
  changeGlobalImageMode("blend")
})
document.getElementById("global-image-view-mode-difference").addEventListener("click", (e) => {
  changeGlobalImageMode("difference")
})

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

function setImageZoom(state) {
  const scale = state.imageZoom.value
  state.imageWrapper.style.transform = `scale(${scale})`
}

function setImageBlend(state) {
  const mode = currentImageMode(state)
  const blend = state.imageBlend.value
  state.images[0].style.opacity = (mode == "blend") ? (1 - blend) : 1
  state.images[1].style.opacity = (mode == "blend") ? blend : 1
}

function currentImageMode(state) {
  for (const imageMode of state.imageModes) {
    if (imageMode.checked) {
      return imageMode.value
    }
  }
}
