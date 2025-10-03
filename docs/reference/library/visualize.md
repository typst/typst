Drawing and data visualization.

If you want to create more advanced drawings or plots, also have a look at the
[CeTZ](https://github.com/johannes-wolf/cetz) package as well as more
specialized [packages]($universe) for your use case.

# Accessibility

All shapes and paths drawn by Typst are automatically marked as
[artifacts]($pdf.artifact) to make them invisible to Assistive Technology (AT)
during PDF export. However, their contents, if any, remain accessible.

If you are using the functions in this model to create an illustration with
semantic meaning, make it accessible by wrapping it in a [`figure`] function
call. Use its [`alt` parameter]($figure.alt) to provide an [alternative description]($guides/accessibility/#textual-representations)
