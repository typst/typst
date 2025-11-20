for (const imageDiff of document.getElementsByClassName("image-diff")) {
    const imageScale = imageDiff.querySelector("input.image-scale")
    const imageWrapper = imageDiff.querySelector(".image-diff-wrapper")
    imageScale.addEventListener("change", (e) => setImageScale(imageWrapper, e.target.value / 100))
    imageScale.addEventListener("input", (e) => setImageScale(imageWrapper, e.target.value / 100))
    setImageScale(imageWrapper, imageScale.value / 100)

    const imageBlend = imageDiff.querySelector("input.image-blend")
    const frontImage = imageDiff.querySelectorAll("img")[1]
    imageBlend.addEventListener("change", (e) => setImageBlend(frontImage, e.target.value / 100))
    imageBlend.addEventListener("input", (e) => setImageBlend(frontImage, e.target.value / 100))
    setImageBlend(frontImage, imageBlend.value / 100)

    // Set opacity of the front image to 1 if not in fade image view mode.
    const imageModeFade = imageDiff.querySelector('input.image-view-mode[value="fade"]')
    for (const imageMode of imageDiff.querySelectorAll("input.image-view-mode")) {
        imageMode.addEventListener("change", (e) => {
            imageModeFadeChanged(frontImage, imageModeFade.checked, imageBlend.value / 100)
        })
    }
    imageModeFadeChanged(frontImage, imageModeFade.checked, imageBlend.value / 100)
}

function setImageScale(imageWrapper, scalePercent) {
    imageWrapper.style.transform = `scale(${scalePercent})`
}

function setImageBlend(image, opacity) {
    image.style.opacity = opacity
}

function imageModeFadeChanged(image, isFade, opacity) {
    setImageBlend(image, isFade ? opacity : 1)
}
