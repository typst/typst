for (const imageDiff of document.getElementsByClassName("image-diff")) {
    const imageWrapper = imageDiff.querySelector(".image-diff-wrapper")
    const frontImage = imageWrapper.querySelectorAll("img")[1]

    const imageModes = imageDiff.querySelectorAll("input.image-view-mode")
    const imageScale = imageDiff.querySelector("input.image-scale")
    const imageAlignXControl = imageDiff.querySelector(".image-align-x-control")
    const imageBlendControl = imageDiff.querySelector(".image-blend-control")
    const imageBlend = imageDiff.querySelector("input.image-blend")

    const state = {
        imageWrapper: imageWrapper,
        frontImage: frontImage,
        imageModes: imageModes,
        imageScale: imageScale,
        imageAlignXControl: imageAlignXControl,
        imageBlendControl: imageBlendControl,
        imageBlend: imageBlend,
    }

    imageScale.addEventListener("change", (e) => setImageScale(state))
    imageScale.addEventListener("input", (e) => setImageScale(state))
    setImageScale(state)

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

function imageModeChanged(state, mode) {
    switch (mode) {
        case "side-by-side": {
            state.imageAlignXControl.disabled = true;
            state.imageBlendControl.disabled = true;
            break;
        }
        case "fade": {
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

function setImageScale(state) {
    const scale = state.imageScale.value
    state.imageWrapper.style.transform = `scale(${scale})`
}

function setImageBlend(state) {
    const mode = currentMode(state)
    const opacity = state.imageBlend.value
    state.frontImage.style.opacity = (mode == "fade") ? opacity : 1
}

function currentMode(state) {
    for (const imageMode of state.imageModes) {
        if (imageMode.checked) {
            return imageMode.value
        }
    }
}
