---
description: |
  Learn how to create accessible documents with Typst. This guide covers semantic markup, reading order, alt text, color contrast, language settings, and PDF/UA compliance to ensure your files work for all readers and assistive technologies.
---

# Accessibility guide

Making a document accessible means that it can be used and understood by everyone. That not only includes people with permanent or temporary disabilities, but also those with different devices or preferences. To underscore why accessibility is important, consider that people might read your document in more contexts than you expected:

- A user may print the document on paper
- A user may read your document on a phone, with reflow in their PDF reader enabled
- A user may have their computer read the document back to them
- A user may ask artificial intelligence to summarize your document for them
- A user may convert your document to another file format like HTML that is more accessible to them

To accommodate all of these people and scenarios, you should design your document for **Universal Access.** Universal Access is a simple but powerful principle: instead of retrofitting a project for accessibility after the fact, design from the beginning to work for the broadest possible range of people and situations. This will improve the experience for all readers!

Typst can help you to create accessible files that read well on screen readers, look good even when reflowed for a different screen size, and pass automated accessibility checkers. However, to create accessible files, you will have to keep some rules in mind. This guide will help you learn what issues impact accessibility, how to design for Universal Access, and what tools Typst gives you to accomplish this. Much of the guidance here applies to all export targets, but the guide focuses on PDF export. Notable differences to HTML export are called out.

## Basics of Accessibility <basics>

Accessible files allow software to do more with them than to just lay them out. Instead, your computer can understand what each part of the document is supposed to represent and use this information to present the document to the user.

This information is consumed by different software to provide access. When exporting a PDF from Typst, the _PDF viewer_ (sometimes also called a reader) will display the document's pages just as you designed them with Typst's preview. Some people rely on _Assistive Technologies_ (AT) such as screen readers, braille displays, screen magnifiers, and more for consuming PDF files. In that case, the semantic information in the file is used to adapt the contents of a file into spoken or written text, or into a different visual representation. Other users will make the PDF viewer reflow the file to create a layout similar to a web page: The content will fit the viewport's width and scroll continuously. Finally, some users will repurpose the PDF into another format, for example plain text for ingestion into a Large Language Model (LLM) or HTML. A special form of repurposing is copy and paste where users use the clipboard to extract content from a file to use it in another application.

Accessibility support differs based on viewer and AT. Some combinations work better than others. In our testing, [Adobe Acrobat][Acrobat] paired with [NVDA][NVDA] on Windows and [VoiceOver][VoiceOver] on macOS provided the richest accessibility support. When using HTML export, browsers provide a more consistent baseline of accessibility when compared to PDF readers.

Only PDF and HTML export produce accessible files. Neither PNGs nor SVGs are accessible on their own. Both formats can be used in an accessible larger work by providing a [textual representation](#textual-representations).

## Maintaining semantics

To add correct semantic information for AT and repurposing to a file, Typst needs to know what semantic role each part of the file plays. For example, this means that a heading in a compiled PDF should not just be text that is large and bold, instead, the file should contain the explicit information (known as a _tag_) that a particular text makes up a heading. A screen reader will then announce it as a heading and allow the user to navigate between headings.

If you are using Typst idiomatically, using the built-in markup and elements, Typst automatically adds tags with rich semantic information to your files. Let's take a look at two code examples:

```example
// ❌ Don't do this
#text(
  size: 16pt,
  weight: "bold",
)[Heading]
```

```example
// ✅ Do this
#show heading: set text(size: 16pt)
= Heading
```

Both of these examples look the same. They both contain the text "Heading" in boldface, sized at 16 point. However, only the second example is accessible. By using the heading markup, Typst knows that the semantic meaning of this text is that of a heading and can propagate that information to the final PDF. In the first example, it just knows that it should use boldface and larger type on otherwise normal text and cannot make the assumption that you meant that to be a heading and not a stylistic choice or some other element like a quote.

Using semantics is not limited to headings. Here are a few more examples for elements you should use:

- Use underscores / [`emph`] instead of the [`text`] function to make text emphasized
- Use stars / [`strong`] instead of the text function to make text carry strong emphasis
- Use lists, including [`terms`], instead of normal text with newlines when working with itemized or ordered content.
- Use [`quote`] for inline and block quotes
- Use the built-in [`bibliography`] and [`cite`] functions instead of manually printing a bibliography
- Use labels and [`ref`] or `@references` to reference other parts of your documents instead of just typing out a reference
- Use the [`caption` argument of the `figure` element]($figure.caption) to provide captions instead of adding them as text below the function call

If you want to style the default appearance of an element, do not replace it with your own custom function. Instead, use [set]($styling/#set-rules), show-set, and [show rules]($styling/#show-rules) to customize its appearance. Here is an example on how you can change how strong emphasis looks in your document:

```example
// Change how text inside of strong emphasis looks
#show strong: set text(tracking: 0.2em, fill: blue, weight: "black")
When setting up your tents, *never forget* to secure the pegs
```

The show-set rule completely changes the default appearance of the [`strong`] element, but its semantic meaning will be conserved. If you need even more customization, you can provide show rules with fully custom layout code while Typst will still be able to track the semantic purpose of the element.

## Reading order

For AT to read the contents of a document in the right order and for repurposing applications, accessible files must make their reading order explicit. This is because the logical reading order can differ from layout order. Floating figures are a common example for such a difference: A figure may be relevant to a paragraph in the center of a page but appear at the top or bottom edge. In non-accessible files, PDF readers and AT have to assume that layout order equals the logical reading order, often leading to confusion for AT users. When the reading order is well-defined, screen readers read a footnote or a floating figure immediately where it makes sense.

Fortunately, Typst markup already implies a single reading order. You can assume that Typst documents will read in the order that content has been placed in the markup. For most documents, this is good enough. However, when using the [`place`] and [`move`] function or [floating figures]($figure.placement), you must pay special attention to place the function call at its spot in the logical reading order in markup, even if this has no consequence on the layout. Just ask yourself where you would want a screen reader to announce the content you are placing.

## Layout containers

Typst provides some layout containers like [`grid`], [`stack`], [`box`], [`columns`], and [`block`] to visually arrange your data. None of these containers come with any semantic meaning attached. Typst will conserve some of these containers (such as columns) during PDF reflow while other containers will be discarded.

When designing for Universal Access, you need to be aware that AT users often cannot view the visual layout that the container creates. Instead, AT will just read its contents, so it is best to think about these containers as transparent in terms of accessibility. For example, a grid will just be announced cell by cell, in the order that you have added cells in the source code. If the layout you created is merely visual and decorative, this is fine. If, however, the layout carries semantic meaning that is apparent to a sighted user viewing the file in a regular PDF reader, it is not accessible. Instead, create an alternative representation of your content that leverages text or wrap your container in the [`figure`] element to provide an alternative textual description.

Do not use the grid container to represent tabular data. Instead, use [`table`]. Tables are accessible to AT users and conserved during reflow and repurposing. When creating tables, use the [`table.header`]($table.header) and [`table.footer`]($table.footer) elements to mark up the semantic roles of individual rows. The table documentation contains an [accessibility section]($table#accessibility) with more information on how to make your tables accessible. Keep in mind that while AT users can access tables, it is often cumbersome to them: Tables are optimized for visual consumption. Being read the contents of a set of cells while having to recall their row and column creates additional mental load. Consider making the core takeaway of the table accessible as text or a caption elsewhere.

Likewise, if you use functions like [`rotate`], [`scale`], and [`skew`], take care that this transformation either has no semantic meaning or that the meaning is available to AT users elsewhere, i.e. in figure [alt text](#textual-representations) or a caption.

## Artifacts <artifacts>

Some things on a page have no semantic meaning and are irrelevant to the content of a document. We call these items _artifacts._ Artifacts are hidden from AT and repurposing and will vanish during reflow. Here are some examples for artifacts:

- The hyphens inserted by automatic hyphenation at the end of a line
- The headers and footers on each page
- A purely decorative page background image

In general, every element on a page must either have some way for AT to announce it or be an artifact for a document to be considered accessible.

Typst automatically tags many layout artifacts such as headers, footers, page back- and foregrounds, and automatic hyphenation as artifacts. However, if you'd like to add purely decorative content to your document, you can use the [`pdf.artifact`] function to mark a piece of content as an artifact. If you are unsure if you should mark an element as an artifact, ask yourself this: Would it be purely annoying if a screen reader announced the element to you? Then, it may be an artifact. If, instead, it could be useful to have it announced, then it is not an artifact.

For technical reasons, once you are in an artifact, you cannot become semantic content ingested by AI again. To stack artifacts and semantic contents, use [`place`] to move the content on top of one another.

Please note that Typst will mark shapes and paths like [`square`] and [`circle`] as artifacts while their content will remain semantically relevant and accessible to AT. If your shapes have a semantic meaning, please wrap them in the [`figure`] element to provide an alternative textual description.

## Color use and contrast

Universal Access not only means that your documents works with AT, reflow, and repurposing, but also that visual access is possible to everyone, including people with impaired eyesight. Not only does aging often come with worse sight, a significant chunk of people have problems differentiating color: About 8% of men and 0.5% of women are colorblind.

<div style="display:flex; gap: 16px;">
<img
  src="chart-bad-regular.png"
  alt="Bar chart showing Energy production in Germany by kind in terawatt-hours on the X axis and the year on the y-axis. Each bar has up to four segments, for Nuclear (violet), Renewables (green), Fossil Fuels (red), and Other (blue). There is a legend in the top right corner associating the segment colors with their labels"
  width="958"
  height="637"
  style="box-shadow: 0 4px 12px rgb(89 85 101 / 20%); width: 500px; max-width: 100%; height: auto; display: block; margin: 24px auto; border-radius: 6px"
>
<img
  src="chart-bad-deuteranopia.png"
  alt="The same bar chart with changed colors, with the segments for Nuclear and Other in a very similar dark blue, and the neighboring segments of Renewables and Fossil Fuels in two almost indistinguishable shades of sickly yellow"
  width="958"
  height="637"
  style="box-shadow: 0 4px 12px rgb(89 85 101 / 20%); width: 500px; max-width: 100%; height: auto; display: block; margin: 24px auto; border-radius: 6px"
>
</div>

This means that color must not be the only way you make information accessible to sighted users in your documents. As an example, consider a stacked bar chart with multiple colored segments per bar. Our example shows a chart of the domestic energy production in Germany by kind[^1]. In the picture, you can see the chart as it would normally appear and a simulation of how it would appear to people with deuteranopia-type color blindness. You can see that the two pairs of the first and last segment both look blue and the center pair looks yellow-ish. The first challenge for the colorblind user is thus to make out the boundary of the "Renewable" and "Fossil Fuels" bar. Then, they must keep track of which bar is which by only their order, adding to their mental load. A way to make this chart even less accessible would be to make the order of segments not match their order in the legend.

How can we improve the chart? First, make sure that no information is solely communicated through color use. One possible way to do this by adding a pattern to each bar. Then, we can help the user make out the boundaries of each segment by adding a high-contrast border. Then, our chart could look something like this:

<div>
<img
  src="chart-good.png"
  alt="The same bar chart with the original colors. This time, black outlines around each segment are added. Additionally, each segment has a unique pattern."
  width="958"
  height="637"
  style="box-shadow: 0 4px 12px rgb(89 85 101 / 20%); width: 500px; max-width: 50%; height: auto; display: block; margin: 24px auto; border-radius: 6px"
>
</div>

This could be further improved by choosing colors that are differentiable to people afflicted by common colorblindness types. There are tools on the web to [simulate the color perception of various color blindnesses][color-blind-simulator]. We aim to add simulation of color blindness to the Typst web app in the future so you can check how your document performs without exporting it. You could also iterate on the design by choosing two-tone patterns, aligning them to the bars, or changing font use.

Also consider the color contrast between background and foreground. For example, when you are using light gray text for footnotes, they could become hard to read. Another situation that often leads to low contrast is superimposing text on an image.

<div>
<img
  src="color-contrast.png"
  alt="Two callout boxes with the text 'Caution: Keep hands away from active stapler' with different designs. Each box has a contrast gauge for its text and graphical elements below it. The left box is shaded in a light red and the text is a regular shade of red. It has a text contrast of 2.8:1 and a graphics contrast of 1.4:1. The right box is white with a red outline and dark red text. It has a text contrast of 5.9:1 and a graphics contrast of 3.9:1."
  width="1536"
  height="708"
  style="box-shadow: 0 4px 12px rgb(89 85 101 / 20%); width: 512px; max-width: 100%; height: auto; display: block; margin: 24px auto; border-radius: 6px"
>
</div>

In our example, we can see two designs for callout boxes. Because these boxes aim to help the user avoid a hazard, it is paramount that they can actually read them. However, in the first box, the background is fairly light, making it hard to make out the box. Worse, the red text is difficult to read on the light red background. The text has a 2.8:1 contrast ratio, failing the bar of 4.5:1 contrast WCAG sets. Likewise, the box has an 1.4:1 contrast ratio with the white page background, falling short of the 3:1 threshold for graphical objects.

Colors in the second example have been adjusted to meet WCAG AA color contrast thresholds. It should be markedly easier to read the text in the box, even if you have good vision!


| Content                                | AA Ratio | AAA Ratio |
|----------------------------------------|---------:|----------:|
| Large text (>=18pt or bold and >=14pt) | 3:1      | 4.5:1     |
| Small text                             | 4.5:1    | 7:1       |
| Non-text content                       | 3:1      | 3:1       |

There are [tools to compare how much contrast a pair of colors has][wcag-contrast] as foreground and background. The most common one is the WCAG color contrast ratio. For a given font size, a color pair may either fail the test, get to the AA level, or reach the higher AAA level. Aim for at least AA contrast for all your color pairings.

Note that common accessibility frameworks like WCAG make an exception for purely decorative text and logos: Due to their graphic character, they can have contrast ratios that fail to achieve AA contrast ratio.

## Textual representations <textual-representations>

To support AT use and some repurposing workflows, all elements with a semantic meaning must have a textual representation. Think about it in terms of Universal Access: If an item is not an [artifact](#artifacts), it has a semantic meaning. If, however, AT cannot ingest the item, the full semantic meaning of a document is not available to AT users. Hence, to provide Universal Access, use the mechanisms built into Typst to provide alternative representations.

When you add an image, be sure to use the [`alt` argument of the image function]($image.alt) to describe what's visible in the image. This alternative description (sometimes known as alt text) should describe the gist of the image: Think about how you would describe the image to a friend if you called them on the phone. To write good alternative descriptions, consider the context in which the image appears:

```example
#image("heron.jpg", alt: "?")

Herons have feet with interdigital webbing, allowing for good mobility when swimming, and wings that span up to 2m30.
```

What could be a good alternative description for [this image][heron]? Let's consider a few examples for what _not_ to do:

- `["Image of a heron"]`: \
  ❌ The screen reader will already announce the image on its own, so saying this is an image is redundant. In this example, the AT user would hear "Image, Image of a heron".

- `["A bird"]`: \
  ❌ The alternative description is not specific enough. For example, it is relevant to a user that the image depicts a heron and both its feet and wings are visible.

- `["Gray heron in flight. Picture by Makasch1966 on Wikimedia Commons, CC Attribution 4.0 International license"]`: \
  ❌ The alternative description should not include details not visible in the image, such as attribution, jokes, or metadata. Keep in mind that it is not accessible to sighted users. That information belongs elsewhere.

- `["Gray heron flying low, heading from the right to left. Its feet are extended and slightly point downwards, touching a blurred horizon where a dark forest becomes visible. The bird's wings are extended and arc upwards. There are out-of-focus branches visible in the lower left corner of the image."]`: \
  ❌ The alternative description is too verbose. Use your discretion and determine how important the image is to the content. Think about how long a sighted user would realistically look at the image; your alt text should take about the same effort to 'consume.' For example, the anatomic description contained above could be appropriate for a longer discussion in a zoology textbook while the compositional information is useful when writing about photography. The context the example image comes with is relatively short, so write a more brief description.

Instead, in the given example, you could use this alternative text:

`["Heron in flight with feet and wings spread"]` \
✅ This alternative description describes the image, is relevant to the context, and matches its brevity.

There are more resources available on the web [to learn more about writing good alternative descriptions][alt-text-tips]. The requirement to add alternative text to images applies to all image formats. Typst does not currently mount the tags of a PDF image into the compiled document, even if the PDF image file on its own was accessible.

Do not use images of text, likewise, do not use the path operations to draw text manually. Typst will not be able to process text in any images to make it accessible in the same way that native text is. There is one exception to this rule: Use an image of text when the appearance of the text is essential to the semantic meaning of the document and cannot be reproduced with Typst natively. In that case, you must describe both the textual content and the essential visual characteristics in the alternative description.

Like the image function, the figure function has a [`alt` attribute]($figure.alt). When you use this attribute, many screen readers and other AT will not announce the content inside of the figure and instead just read the alternative description. Your alternative description must be comprehensive enough so that the AT user does not need to access the children of the figure. Only use the alternative description if the content of the figure are not otherwise accessible. For example, do not use the `alt` attribute of a figure if it contains a `table` element, but do use it if you used shapes within that come with a semantic meaning. If you specify both `alt` and `caption`, both will be read by AT. If your figure contains an image, it suffices to set an alternative description on the image.

```typ
#figure(
  alt: "Star with a blue outline",
  curve.with(
    stroke: blue,
    curve.move((25pt, 0pt)),
    curve.line((10pt, 50pt)),
    curve.line((50pt, 20pt)),
    curve.line((0pt, 20pt)),
    curve.line((40pt, 50pt)),
    curve.close(),
  ),
)
```

Finally, you can specify an alternative description on math using [`math.equation`]. Describe your formula as if read out loud in natural language. Currently, adding an alternative description is required for accessible math for all export formats. In the future, Typst make math automatically accessible in HTML and PDF 2.0 by leveraging MathML technology. Not adding an alternative description for your formula will result in a failure of PDF/UA-1 export.

```typ
#math.equation(
  alt: "a squared plus b squared equals c squared",
  $ a^2 + b^2 = c^2 $,
)
```

Another element that represents itself as text are links. It is best to avoid non-descriptive link texts such as _here_ or _go._ These link texts also hurt Search Engine Optimization (SEO) if that is a consideration for your document. Instead, try to have the link contain text about where it is pointing to. Note that, unless you are aiming for the highest level of accessibility, it is also okay if the link itself is not descriptive but its purpose can be understood from the content immediately surrounding it.

## Natural Language

In order for screen readers to pronounce your document correctly and translation software to work properly, you must indicate in which natural language your document is written. Use the rule [`[#set text(lang: "..")]`]($text.lang) at the very start of your document or your template's capability to set a language. If you do not do so, Typst will assume that your content is in English. The natural language you choose not only impacts accessibility, but also how Typst will apply hyphenation, what typesetting conventions are applied, the labels of figures and references, and, in the web app, what language is used for spellcheck.

If you are using a language with significant variation between regions, such as Chinese or English, also use [the `region` argument]($text.region). For example, Chinese as it is spoken in Hong Kong would look like this:

```typ
#set text(lang: "zh", region: "HK")
```

To specify your language, use ISO 639 codes. For regions, use the [ISO 3166-1 alpha-2][iso-3166-1-alpha-2] code. ISO 639 contains three variants, one for two-letter language codes like "de" for German [(ISO 639-1)][iso-639-1-list] and two for three-letter language codes like "deu" ([ISO 639-2][iso-639-2-list] and [ISO 639-3][iso-639-3-list]). If your language has a two-letter ISO 639-1 code, always prefer using that. ISO 639-2 and 639-3 share most codes, but there are some differences. When your language code differs between the two standards, use ISO 639-2 when exporting to PDF 1.7 (Typst's default) and below and ISO 639-3 for export to PDF 2.0 and HTML.

There are three special language codes defined by both ISO 639-2 and ISO 639-3 that you can use when providing a normal language code is difficult:

- `zxx` for text that is not in a natural language
- `und` for text for which you cannot determine the natural language
- `mis` for text in languages that have not been assigned a language code

If your document contains text in multiple languages, you can use the text function to enclose instances of other languages:

```example
This is #text(lang: "fr")[français].
```

## Document Title and Headings

Titling your document makes it easier to retrieve it and to navigate between it and other documents, both for AT users and regular users of PDF viewers. This is why accessibility standards such as WCAG and PDF/UA require you to set a machine-readable title for your document.

To do so in Typst, place this set rule in your document before any content:

```typ
#set document(title: "GlorboCorp Q1 2023 Revenue Report")
```

This will set the [title in the document's metadata]($document.title) and in the title bar of the PDF viewer or a web browser. If this results in an error when using a template, consider whether your template may provide an alternative way to set the document title.

Most likely, you will also want to include the title in your document. To do so, use the [`title`] element. When you add a call to the title element without any arguments, it will print the contents of what you set as the document's title. Alternatively, you can customize the title by passing content as the positional body argument. Do not use the title element more than once in your document.

Never use a heading for your document title, instead, use the title element. Should you have experience with HTML, it is important to remember that the semantics of the heading element in Typst differ from HTML headings. It is encouraged to use multiple first-level headings for section headings in Typst documents. When exporting to HTML, a title will be serialized as a `h1` tag while a first-level heading will be serialized as a `h2` tag. In PDF export, the title and headings will be correctly tagged based on the PDF version targeted.

It is important that the sequence of headings you use is sequential: Never skip a heading level when going deeper. This means that a third-level heading must be followed by a heading of level four or lower, but never a heading of level five or higher.

```typ
// ❌ Don't do this:
= First level heading
=== Third level heading
```

Note that in order to pass the [automated accessibility check in Adobe Acrobat][acro-check-outline], documents with 21 pages or more must contain outlined headings.

## Accessibility Standards and Legislation

There are international standards that help you to assert that a Typst document is accessible. For PDF export, the Typst compiler can check whether you are complying with PDF-based accessibility standards and assert the compliance in the compiled file:

- **Tagged PDF:** Tagged PDF contain machine-readable data about the semantic structure of a document that AT can parse. Typst will write Tagged PDFs by default, but keep in mind that Typst can only write appropriate tags if it knows about the semantic structure of your document. Refer to the Section Maintaining semantics to learn how to use Typst's elements to communicate element semantics. To provide Universal Access, you are also responsible to provide textual representation of non-text content yourself.

- **PDF/A-2a** and **PDF/A-3a:** The PDF/A standard describe how to produce PDF files that are best suited for archival. Parts two and three of the PDF/A standard feature multiple conformance levels. The strictest conformance level A contains rules for accessibility as only they remain usable to the broadest range of people in the far future. Level A implies conformance with Tagged PDF, forces you to provide alternative description for images, and disallows the use of characters in the [Unicode Private Use Area][unic-pua] whose meaning is unclear to the general public. Other PDF/A rules not relating to accessibility, e.g. about colors and more also apply. When choosing between the two standards, choose PDF/A-2a unless you need to attach other PDF files. Conformance level A has been removed from PDF/A-4 in favor of the dedicated PDF/UA standard. When targeting PDF 2.0, use PDF/A-4 together with PDF/UA-2 instead (the latter is not yet supported by Typst).

- **PDF/UA-1:** The PDF/UA standard explains how to write a PDF 1.7 file optimized for Universal Access. It implies Tagged PDF, forces alternative descriptions for images and mathematics, requires a document title, and introduces rules how document contents like tables should be structured. If you are following this guide, you should be in compliance with most rules in PDF/UA-1 already.

To enable either PDF/A-2a, PDF/A-3a, or PDF/UA, use the export dropdown in the web app and click on PDF or the [appropriate CLI flag]($pdf).

There are also a more recent part of the PDF/UA standard that targets PDF 2.0 files, PDF/UA-2. Support for PDF/UA-2 not yet available in Typst. [Both parts of the PDF/UA specification are available free of charge from the PDF Association.][pdf-ua-free] The industry standard [Well Tagged PDF (WTPDF)][WTPDF] is very similar to PDF/UA-2. All files conforming to WTPDF can also declare conformance with PDF/UA-2.

When you select one or multiple of these standards for PDF export, Typst will detect if you are in violation of their rules and fail the export with a descriptive error message. You can combine multiple standards. For the strictest accessibility check currently available, choose PDF/UA-1. You can combine it with PDF/A-2a for the broadest possible range of checks. Do not disable writing tags unless you have a good reason, as they provide a baseline of accessibility across all documents you export.

Maybe you already noticed that some of the factors that go into Universal Access are hard to check automatically. For example, Typst will currently not automatically check that your color contrasts are sufficient or whether the natural language set matches the actual natural language (although the amount of spellcheck errors should provide a hint if you are using the web app). There are two international standards that address some of these human factors in more detail:

- The **[Web Content Accessibility Guidelines (WCAG)][WCAG]**: Designed by the W3C, a big international consortium behind the technologies that power the internet, WCAG describes how to make a web site accessible. All of these rules are applicable to Typst's HTML output, and many of them apply to its PDF output. WCAG separates its rules into the three levels A, AA, and AAA. It is recommended that normal documents aim for AA. If you have high standards for Universal Access, you can also consider AAA Success Criteria. However, Typst does not yet expose all PDF features needed for AAA compliance, e.g. an AT-accessible way to define expansions for abbreviations.
- The **[European Norm EN 301 549][EN301549]**: Its Section 9 describes how to create accessible websites and its Section 10 describes what rules apply to non-web documents, including PDFs created by Typst. It points out which WCAG clauses are also applicable to PDFs. Conformance with this standard is a good start for complying with EU and national accessibility laws.

Keep in mind that in order to conform with EN 301 549 and the relevant WCAG provisions, your document must be tagged. If you aim for conformance, we strongly suggest using PDF/UA-1 for export to automate many of the checks for the success criteria within.

Many territories have accessibility legislation that requires you to create accessible files under some circumstances. Here are only some of them:

- **[European Accessibility Act (EAA, EU 2019/882)][EAA]**: This regulation applies to e-books, consumer banking services, e-commerce services, and more. It requires the files distributed in these applications to be accessible.
- **[Americans with Disabilities Act (ADA) Title II][ADA-2]**: This amendment of the ADA requires public sector organizations to provide files in accordance to WCAG by 2026.

Using this guide can help you reach compliance with either regulation.

## Testing for Accessibility

In order to test whether your PDF document is accessible, you can use automated tools and manual testing. Some standards like PDF/UA and PDF/A can be checked exclusively through automated tools, while some rules in WCAG and other standards require manual checks. Many of the automatable checks are automatically passed by Typst when Tagged PDF is enabled. For many other automatable checks, you can enable PDF/UA-1 export so that Typst will run them instead. Automated tools can only provide a baseline of accessibility, for truly Universal Access, it is best if you try the document yourself with AT.

Here is a list of automated checkers to try to test for conformance:

- **[veraPDF][veraPDF]:** This open-source tool can check if your PDF file conforms to the parts of the PDF/A and PDF/UA standards it declared conformance with. Use this tool if you have chosen one of these standards during export. Failures are considered bugs in Typst and should be reported on GitHub.

- **[PDF Accessibility Checker (PAC)][PAC]:** The freeware PAC checks whether your document complies with PDF/UA and WCAG rules. When you receive a hard error in the PDF/UA tab, this is considered a bug in Typst and should be reported on GitHub. Warnings in the PDF/UA and Quality tabs may either be bugs, problems in your document, or neither. Check on the [Forum][Typst Forum] or on [Discord][Discord] if you are unsure. Errors and warnings in the WCAG tab indicate problems with your document.

- **[Accessibility Check in Adobe Acrobat Pro][acro-check]:** The accessibility checker in the paid version of Adobe Acrobat checks all PDF documents for problems. Instead of checking compliance with a well-known international or industry standard, Adobe has created their own suite of tests. Because the rules behind these tests sometimes contradict international standards like PDF/UA, some of Acrobat's checks are expected to fail for Typst documents[^2]. Other checks, such as the contrast check are useful and indicate problems with your document.

When doing manual checking, you can start with a checklist. If your organization places emphasis on accessibility, they will sometimes have their own list. In absence of one, you can try lists by universities such as [Universität Bremen (in English)][checklist-unib] or governments such as in [Canada][checklist-canada] or by the [US Social Security Administration][checklist-us-ssa]. Although these checklists differ in verbosity, they all cover the most essential manual checks. Many of the technical checks in them can be skipped if you choose PDF/UA-1 export in Typst. If unsure which checklist to use, choose one from an organization culturally similar to yours.

However, to reach the highest standard of accessibility for widely circulated documents, consider checking your document with AT. Although there are many AT and PDF viewers, it is sufficient to test a single combination. Which is best differs depending on your operating system:

- Windows: Test with [Adobe Acrobat][Acrobat] and [NVDA][NVDA]. NVDA is free, open-source software. A free version of Acrobat is available.
- macOS: Test with [Adobe Acrobat][Acrobat] and [VoiceOver][VoiceOver]. VoiceOver is the screen reader that comes with macOS and other Apple platforms.
- Linux: Test with [Evince][Evince] or [Okular][Okular] and [Orca][Orca]. All three tools are free, open-source software. However, AT support across Linux platforms lags behind what is available on Windows and macOS. Likewise, Evince and Okular have less accessibility support than Acrobat. We strongly suggest testing with Acrobat instead.

When first getting into testing, consider completing the interactive training program your screen reader offers, if any. Building confidence with a screen reader helps you experience your document like a full-time screen reader user. When checking your document, check that it not only makes all the same information accessible that is available to a sighted user, but also that it is easy to navigate. The experience your users will have will vary based on the pairing of PDF viewer and AT they use.

## Limits and considerations for export formats

Even when you design your document with accessibility in mind, you should be aware of the limitations of your export format. Fundamentally, AT support for PDF files is more difficult to implement than for other formats such as HTML. PDF was conceived in 1993 to accurately render print documents on a computer. Accessibility features were first added with PDF 1.4 in 2001, and improved in PDF 1.5 (2003) and PDF 2.0 (2017). By contrast, HTML offers a richer semantic model and more flexibility, so AT support in browsers generally surpasses what is possible in PDF viewers.

Also keep in mind that PDF files are mostly static. This allows you to disregard many WCAG and EN 301 549 rules designed for interactive content and multimedia. However, the lack of interactivity also makes it more difficult for users to customize a document's layout to their needs.

For example, [WCAG Success Criterion 1.4.12][wcag-sg-1412-us] (codified in Clause 10.1.4.12 of EN 301 549) prescribes that a user must be able to increase character, letter, line, and paragraph spacing to very wide values. This benefits users with reduced vision or dyslexia. The Success Criterion does not require you to design your document with these layout parameters, instead, it only requires a mechanism through which users can increase these parameters when reading the document. For HTML files, it is easy to comply with this Success Criterion because the browser lets the user override these spacing parameters on a page. For PDF, the situation is more nuanced: Theoretically, Typst adds tags and attributes designed for reflow to a file. A PDF reader, when reflowing, could allow its user to increase spacings beyond what is codified in these tags. In practice, we are not aware of a PDF viewer with this feature. Instead, this Success Criterion can be satisfied by repurposing the PDF into a HTML file and opening it in a browser.

In practice, even if your file is technically compliant, you cannot expect your users to know about these workarounds. Therefore, if you are aiming to meet the highest standards of Universal Access, consider distributing an HTML version of your document alongside your PDF. Export this file directly using Typst's [HTML export]($html) (in preview). Even though HTML export will not conserve many aspects of your visual layout, it will produce a file that leverages semantic HTML and technologies like [Digital Publishing ARIA][dpub-aria] to provide Universal Access. It will be of a higher quality than a PDF file repurposed to HTML.

Finally, keep in mind that PDFs are designed for print. Hence, you should not assume that interactive features like links are available to users who chose to print your document.

As mentioned above, files created by PNG and SVG export are not accessible.

[^1]: Dataset from the German Federal Statistics Authority (Statistisches Bundesamt, Destatis). ["Bruttostromerzeugung nach Energieträgern in Deutschland ab 1990"](https://www.destatis.de/DE/Themen/Branchen-Unternehmen/Energie/Erzeugung/bar-chart-race.html), 2025, available under the _Data licence Germany – attribution – version 2.0._

[^2]: For example, when using footnotes, the check "Lbl and LBody must be children of LI" in the "List" section is expected to fail

[NVDA]: https://www.nvaccess.org/download/
[Acrobat]: https://www.adobe.com/acrobat.html
[VoiceOver]: https://support.apple.com/guide/voiceover/welcome/mac
[wcag-contrast]: https://webaim.org/resources/contrastchecker/ "WebAIM Contrast Checker"
[wcag-sg-1412-us]: https://www.w3.org/WAI/WCAG21/Understanding/text-spacing.html "Understanding SC 1.4.12: Text Spacing (Level AA)"
[alt-text-tips]: https://webaim.org/techniques/alttext/
[iso-3166-1-alpha-2]: https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2 "ISO 3166-1 alpha-2"
[iso-639-1-list]: https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes "List of ISO 639 language codes"
[iso-639-2-list]: https://en.wikipedia.org/wiki/List_of_ISO_639-2_codes "List of ISO 639-2 codes"
[iso-639-3-list]: https://en.wikipedia.org/wiki/List_of_ISO_639-3_codes "List of ISO 639-3 codes"
[WCAG]: https://www.w3.org/TR/WCAG21/
[EN301549]: https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf
[EAA]: https://eur-lex.europa.eu/eli/dir/2019/882/oj "Directive (EU) 2019/882 of the European Parliament and of the Council of 17 April 2019 on the accessibility requirements for products and services (Text with EEA relevance)"
[ADA-2]: https://www.ada.gov/law-and-regs/regulations/title-ii-2010-regulations/ "Americans with Disabilities Act Title II Regulations"
[color-blind-simulator]: https://daltonlens.org/colorblindness-simulator "Online Color Blindness Simulators"
[unic-pua]: https://en.wikipedia.org/wiki/Private_Use_Areas "Private Use Areas"
[pdf-ua-free]: https://pdfa.org/sponsored-standards/ "Sponsored ISO standards for PDF technology"
[WTPDF]: https://pdfa.org/wtpdf/ "Well-Tagged PDF (WTPDF)"
[acro-check]: https://helpx.adobe.com/acrobat/using/create-verify-pdf-accessibility.html "Create and verify PDF accessibility (Acrobat Pro)"
[acro-check-outline]: https://helpx.adobe.com/acrobat/using/create-verify-pdf-accessibility.html#Bookmarks "Create and verify PDF accessibility (Acrobat Pro) - Bookmarks"
[veraPDF]: https://verapdf.org "Industry Supported PDF/A Validation"
[PAC]: https://pac.pdf-accessibility.org/en "PDF Accessibility Checker"
[Typst Forum]: https://forum.typst.app/
[Discord]: https://discord.gg/2uDybryKPe
[checklist-unib]: https://www.uni-bremen.de/fileadmin/user_upload/universitaet/Digitale_Transformation/Projekt_BALLON/Checklisten/2._Auflage_englisch/Checklist_for_accessible_PDF_ENG-US_ver2.pdf "Accessible E-Learning and Teaching - Checklist for Creating and Reviewing Accessible PDF Documents"
[checklist-canada]: https://a11y.canada.ca/en/pdf-accessibility-checklist/ "PDF accessibility checklist"
[checklist-us-ssa]: https://www.ssa.gov/accessibility/checklists/PDF_508_Compliance_Checklist.pdf
"Portable Document Format (PDF) Basic Testing Guide"
[Evince]: https://wiki.gnome.org/Apps/Evince/
[Okular]: https://okular.kde.org/ "Okular - The Universal Document Viewer"
[Orca]: https://orca.gnome.org "Orca - A free and open source screen reader"
[heron]: https://commons.wikimedia.org/wiki/File:Reiher_im_Flug.jpg
[dpub-aria]: https://www.w3.org/TR/dpub-aria-1.1/ "Specification for Digital Publishing WAI-ARIA Module 1.1"
