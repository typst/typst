const diffs = []
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
        imageWrapper: imageWrapper,
        images: images,
        imageModes: imageModes,
        imageZoom: imageZoom,
        imageAlignXControl: imageAlignXControl,
        imageBlendControl: imageBlendControl,
        imageBlend: imageBlend,
    }
    diffs.push(state)

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

    const mode = currentMode(state);
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
    for (const state of diffs) {
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
    const mode = currentMode(state)
    const blend = state.imageBlend.value
    state.images[0].style.opacity = (mode == "blend") ? (1 - blend) : 1
    state.images[1].style.opacity = (mode == "blend") ? blend : 1
}

function currentMode(state) {
    for (const imageMode of state.imageModes) {
        if (imageMode.checked) {
            return imageMode.value
        }
    }
}
