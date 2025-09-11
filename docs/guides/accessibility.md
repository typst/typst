---
description: |
  Learn how to create accessible documents with Typst. This guide covers semantic markup, reading order, alt text, color contrast, language settings, and PDF/UA compliance to ensure your files work for all readers and assistive technologies.
---

# Accessibility guide

Making a document accessible means that it can be used and understood by everyone. That not only includes people with permanent or temporary disabilities, but also those with different devices or preferences. To underscore why accessibility is important, consider that people might read your document in more contexts than you expected:

- A user may print the document on paper
- A user may read your document on a phone, with reflow in their PDF reader enabled
- A user may have their computer read the document back to you
- A user may ask an Artificial Intelligence to summarize your document to them
- A user may convert their document to another file format like HTML that is more accessible to them

To accommodate all of these people and scenarios, you should design your document for **Universal Access.** Universal Access is a simple but powerful principle: instead of retrofitting a project for accessibility after the fact, design from the beginning to work for the broadest possible range of people and situations. This will improve the experience for all readers!

Typst can help you to create accessible files that read well on screen readers, look good even when reflowed for a different screen size, and pass automated accessibility checkers. However, to create accessible files, you will have to keep some rules in mind. This guide will help you to learn what issues impact accessibility, how to design for Universal Access, and what tools Typst gives you to accomplish this. Much of the guidance here applies to all export targets, but the guide focusses on PDF export. Notable differences to HTML are called out.

## Basics of Accessibility

Accessible files allow software to do more with them than to just lay them out. Instead, your computer can understand what each part of the document is supposed to represent and use this information to present the document to the user.

This information is consumed by different software to provide access. When exporting a PDF from Typst, the _PDF viewer_ (sometimes also called a reader) will display the document’s pages just as you designed them with Typst’s preview. Some people rely on _Assistive Technologies_ (AT) such as screen readers, braille displays, screen magnifiers, and more for consuming PDF files. In that case, the semantic information in the file is used to adapt the contents of a file into spoken or written text, or into a different visual representation. Other users will make the PDF viewer reflow the file to create a layout similar to a web page: The content will fit the viewport’s width and scroll continuously. Finally, some users will repurpose the PDF into another format, for example plain text for ingestion into a Large Language Model (LLM) or HTML. A special form of repurposing is copy and paste where users use the clipboard to extract content from a file to use it in another application.

Accessibility support differs based on viewer and AT. Some combinations work better than others. In our testing, Adobe [Acrobat] paired with [NVDA] on Windows and [VoiceOver] on macOS provided the richest accessibility support.

## Maintaining semantics

To add correct semantic information for AT and repurposing to a file, Typst needs to know what semantic part each part of the file plays. For example, this means that a heading in a compiled PDF should not just be text that is large and bold, instead, the file should contain the explicit information (known as a _tag_) that a particular text makes up a heading. A screen reader will then announce it as a heading and allow the user to navigate between headings.

If you are using Typst idiomatically, using the built-in markup and elements, Typst automatically adds tags with rich semantic information to your files. Let’s take a look at two code examples:

```example
#text(size: 16pt, weight: "bold")[Heading]
```

```example
#show heading: set text(size: 16pt)
= Heading
```

Both of these examples look the same. They both contain the text "Heading" in boldface, sized at 16 point. However, only the second example is accessible. By using the heading markup, Typst knows that the semantic meaning of this text is that of a heading and can propagate that information to the final PDF. In the first example, it just knows that it should use boldface and larger type on otherwise normal text and cannot make the assumption that you meant that to be a heading and not a stylistic choice or some other element like a quote.

Using semantics is not limited to headings. Here are a few more examples for elements you should use:

- Use underscores / [`emph`]($emph) instead of the text function to make text emphasized
- Use stars / [`strong`]($strong) instead of the text function to make text carry strong emphasis
- Use lists, including [`terms`]($terms), instead of normal text with newlines when working with itemized or ordered content.
- Use [`quote`]($quote) for inline and block quotes
- Use the built-in [`bibliography`]($bibliography) and [`cite`]($cite) functions instead of manually printing a bibliography
- Use labels and [`ref`]($ref) or `@references` to reference other parts of your documents instead of just typing out a reference
- Use the [`caption` argument of the `figure` element]($figure.caption) to provide captions instead of adding them as text below the function call

If you want to style the default appearance of an element, do not replace it with your own custom function. Instead, use [set]($styling/#set-rules), show set, and [show rules]($styling/#show-rules) to customize its appearance. Here is an example on how you can change how strong emphasis looks in your document:

```example
// Change how text inside of strong emphasis looks
#show strong: set text(tracking: 0.2em, fill: blue, weight: "black")
When setting up your tents, *never forget* to secure the pegs
```

The show set rule completely changes the default appearance of the strong element, but its semantic meaning will be conserved. If you need even more customization, you can provide show rules with fully custom layouting code while Typst will still be able to track the semantic purpose of the element.

## Reading order

For AT to read the contents of a document in the right order and for repurposing applications, accessible files must make their reading order explicit. This is because the logical reading order can differ from layout order. Floating figures are a common example for such a difference: A figure may be relevant to a paragraph in the center of a page but appear at the top or bottom edge. In non-accessible files, PDF readers and AT have to assume that layout order equals the logical reading order, often leading to confusion for AT users. Instead, when reading order is defined screen readers read a footnote or a floating figure immediately where it makes sense.

Fortunately, Typst markup already implies a single reading order. You can assume that Typst documents will read in the order that content has been placed in the markup. For most documents, this is good enough. However, when using the place and move function or floating figures, you must pay special attention to place the function call at its spot in the logical reading order in markup, even if this has no consequence on the layout. Just ask yourself where you’d want a screen reader to announce the content you are placing.

## Layout containers

Typst provides some layout containers like [`grid`]($grid), [`stack`]($stack), [`box`]($box), [`columns`]($columns), and [`block`]($block) to visually arrange your data. None of these containers come with any semantic meaning attached. Typst will conserve some of these containers (such as columns) during PDF reflow while other containers will be discarded.

When designing for Universal Access, you need to be aware that AT users often cannot view the visual layout that the container creates. Instead, AT will just read its contents, so it is best to think about these containers as transparent in terms of accessibility. For example, a grid will just be announced cell by cell, in the order that you have added cells in the source code. If there layout you created is merely visual and decorative, this is fine. If, however, the layout carries semantic meaning that is apparent to a sighted user viewing the file in a regular PDF reader, it is not accessible. Instead, create an alternative representation of your content that leverages text or wrap your container in the [`figure`]($figure) element to provide an alternative textual description.

Do not use the grid container to represent tabular data. Instead, use [`table`]($table). Tables are accessible to AT users and conserved during reflow and repurposing. When creating tables, use the [`table.header`]($table.header) and [`table.footer`]($table.footer) elements to mark up the semantic roles of individual rows. Keep in mind that while AT users can access tables, it is often cumbersome to them: Tables are optimized for visual consumption. Being read the contents of a set of cells while having to recall their row and column creates additional mental load. Consider making the core takeaway of the table accessible as text or a caption elsewhere.

Likewise, if you use functions like [`rotate`]($rotate), [`scale`]($scale), and [`skew`]($skew), take care that this transformation either has no semantic meaning or that the meaning is available to AT users elsewhere, i.e. in figure alt text or a caption.

## Artifacts

Some things on a page have no semantic meaning and are irrelevant to the content of a document. We call these items _artifacts._ Artifacts are hidden from AT and repurposing and will vanish during reflow. Here are some examples for artifacts:

- The hyphens inserted by automatic hyphenation at the end of a line
- The headers and footers on each page
- A purely decorative page background image

In general, every element on a page must either have some way for AT to announce it or be an artifact for a document to be considered accessible.

Typst automatically tags many layout artifacts such as headers, footers, page back- and foregrounds, and automatic hyphenation as artifacts. However, if you’d like to add purely decorative content to your document, you can use the `pdf.artifact` function to mark a piece of content as an artifact. <!-- TODO: Link once it exists -->

Please note that Typst will mark shapes and paths like [`square`]($square) and [`oval`]($oval) as artifacts while their content will remain semantically relevant and accessible to AT. If your shapes have a semantic meaning, please wrap them in the [`figure`]($figure) element to provide an alternative textual description.

## Color use and contrast

Universal Access not only means that your documents works with AT, reflow, and repurposing, but also that visual access is possible to everyone, including people with impaired eye sight. Not only does aging often come with worse sight, a significant chunk of people have problems differentiating color: About 8% of men and 0.5% of women are color blind.

This means that color must not be the only way you make information accessible to sighted users in your documents. As an example, consider a stacked bar chart with multiple colored segments per bar. For a color blind user, there are multiple challenges here. The first is to associate the color of a bar segment with its entry in the legend. You could help the reader by always ensuring that the colors appear in the same order as they do in the legend and note this. The next challenge is to identify the boundaries of the colored segments in a single bar. To make this easier, you could draw a dark or light boundry with high contrast. Alternatively, you could address both problems by also giving each segment a unique pattern in addition to a color.

There are tools on the web to simulate the color perception of various color blindnesses. We aim to add simulation of color blindness to the Typst web app in the future so you can check how your document performs without exporting it.

Also consider the color contrast between background and foreground. For example, when you are using light gray text for footnotes, they could become hard to read. Another situation that often leads to low contrast is superimposing text on an image. There are [tools to compare how much contrast a pair of colors has][wcag-contrast] as foreground and background. The most common one is the WCAG color contrast ratio. For a given font size, a color pair may either fail the test, get to the AA level, or reach the higher AAA level. Aim for at least AA contrast for all your color pairings.

| Content                                | AA Ratio | AAA Ratio |
|----------------------------------------|----------|-----------|
| Large text (>=18pt or bold and >=14pt) | 3:1      | 4.5:1     |
| Small text                             | 4.5:1    | 7:1       |
| Non-text content                       | 3:1      | 3:1       |

Note that common accessibility frameworks like WCAG make an exception for purely decorative text and logos: Due to their graphic character, they can have contrast ratios that fail to achieve AA contrast ratio.

## Text layout and spacing

Some users that use your document visually, such as users with low vision and dyslexia, may require larger spacing between characters, words, lines, and paragraphs. It is generally accepted that you can freely choose these values to fit your design intent as long as the final user of your document can change these values. In principle, Typst emits PDFs that allow viewers to do this by either overriding these values in reflow mode or by repurposing the file into another format such as HTML, where browsers allow the user to change these values.

In practice, most PDF viewers do not support overriding these layout properties, even when they support reflow. If you expect your users to face challenges with text layout, consider exporting your document to HTML which better accommodates layout customization instead.

Alternatively, you can add these set rules to a version of your document to meet the recommendations of [WCAG Success Criterion 1.4.12][wcag-sg-1412-us] without any user interaction:

```typ
// Set letter and word spacing
#set text(spacing: 0.16em, tracking: 0.12em)

// Add 1.5x of the normal line spacing
// and 2x the font size as paragraph spacing
#set par(leading: 1.3em, spacing: 2em)
```

All values given in these set rules are minimums. When you use these values, consider distributing two versions of your document, as documents with spacing this large are not visually appealing to everyone.

## Textual representations

To support AT use and some repurposing workflows, all elements with a semantic meaning must have a textual representation. Think about it in terms of Universal Access: If an item is not an artifact, it has a semantic meaning. If, however, AT cannot ingest the item, the full semantic meaning of a document is not available to AT users. Hence, to provide Universal Access, use the mechanisms built into Typst to provide alternative representations.

When you add an image, be sure to use the [`alt` argument of the image function]($image.alt) to describe what’s visible in the image. This alternative description (sometimes known as alt text) should describe the gist of the image: Think about you would describe the image to a friend if you called them on the phone. There are resources available on the web [to learn more about writing good alternative descriptions][alt-text-tips]. The requirement to add alternative text to images applies to PDF and SVG images. Typst does not currently mount the tags of a PDF image into the compiled document, even if the PDF image file on its own was accessible.

Like the image function, the figure function has a [`alt` attribute]($figure.alt). When you use this attribute, many screen readers and other AT will not announce the content inside of the figure and instead just read the alternative description. Your alternative description must be comprehensive enough so that the AT user does not need to access the children of the figure. Only use the alternative description if the content of the figure are not otherwise accessible. For example, do not use the `alt` attribute of a figure if it contains a `table` element, but do use it if you used shapes within that come with a semantic meaning. If your figure contains an image, it suffices to set an alternative description on the image.

Do not use images of text, likewise, do not use the path operations to draw text manually. Typst will not be able to process text in images to make it accessible in the same way that native text is. The only exceptions to this rule are images in which the appearance of the text is essential to the semantic meaning of the document and cannot be reproduced with Typst natively. In that case, you must describe both the text content and the essential visual characteristics in the alternative description.

## Natural Language

In order for screen readers to pronounce your document correctly and translation software to work properly, you must indicate in which natural language your document is written. Use the set rule [`#set text(lang: "..")`]($text.lang) at the very start of your document or your template’s capability to set a language. If you do not do so, Typst will assume that your content is in English. The natural language you choose not only impacts accessibility, but also how Typst will apply hyphenation, what text layout conventions are applied, the labels of figures and references, and, in the web app, what language is used for spellcheck.

If you are using a language with significant variation between regions, such as Chinese or English, also use [the `region` argument]($text.region). For example, Chinese as it is spoken in Hong Kong would look like this:

```typ
#set text(lang: "zh", region: "HK")
```

To specify your language, use ISO 639 codes. For regions, use the [ISO 3166-1 alpha-2][iso-3166-1-alpha-2] code. ISO 639 contains three variants, one for two-letter language codes like "de" for German [(ISO 639-1)][iso-639-1-list] and two for three-letter language codes like "deu" ([ISO 639-2][iso-639-2-list] and [ISO 639-3][iso-639-3-list]). If your language has a two-letter ISO 639-1 code, always prefer using that. ISO 639-2 and 639-3 share most codes, but there are some differences. When your language code differs between the two standards, use ISO 639-2 when exporting to PDF 1.7 (Typst’s default) and below and ISO 639-3 for export to PDF 2.0 and HTML.

There are three special language codes defined by both ISO 639-2 and ISO 639-3 that you can use when providing a normal language code is difficult:

- `zxx` for text that is not natural language
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

This will set the [title in the document’s metadata]($document.title) and in the title bar of the PDF viewer. If this results in an error when using a template, consider whether your template may provide an alternative way to set the document title.

Most likely, you will also want to include the title in your document. To do so, use the [`title`]($title) element. When you add a call to the title element without any arguments, it will print the contents of the document’s title. Alternatively, you can customize the title by passing content as the positional body argument. Do not use the title element more than once in your document.

Never use a heading for your document title, instead, use the title element. Should you have experience with HTML, it is important to remember that the semantics of the heading element in Typst differ from HTML headings. It is encouraged to use multiple first-level headings for section headings in Typst documents. When exporting to HTML, a title will be serialized as a `h1` tag while a first-level heading will be serialized as a `h2` tag. In PDF export, the title and headings will be correctly tagged based on the PDF version targeted.

It is important that the sequence of headings you use is sequential: Never skip a heading level when going deeper. This means that a third-level heading must be followed by a heading of level four or lower, but never a heading of level five or higher.

```typ
// Don’t do this:
= First level heading
=== Third level heading
```

## Accessibility Standards and Legislation

There are international standards that help you to assert that a Typst document is accessible. For PDF export, Typst can check whether you are complying with PDF-based accessibility standards and assert the compliance in the compiled file:

- **Tagged PDF:** Tagged PDF contain machine-readable data about the semantic structure of a document that AT can parse. Typst will write Tagged PDFs by default, but keep in mind that Typst can only write appropriate tags if it knows about the semantic structure of your document. Refer to the Section Maintaining semantics to learn how to use Typst’s elements to communicate element semantics. To provide Universal Access, you are also responsible to provide textual representation of non-text content yourself.
- **PDF/A-2a** and **PDF/A-3a:** The PDF/A standard describe how to produce PDF files that are best suited for archival. Parts two and three of the PDF/A standard feature multiple conformance levels. The strictest conformance level A contains rules for accessibility as only they remain usable to the broadest range of people in the far future. Level A implies conformance with Tagged PDF, forces you to provide descriptions text for images, and disallows the use of characters in the Unicode Private Use Area whose meaning is unclear to the general public. Other PDF/A rules not relating to accessibility, e.g. about colors and more also apply. When choosing between the two standards, choose PDF/A-2a unless you need to attach other PDF files. Conformance level A has been removed from PDF/A-4 in favor of the dedicated PDF/UA standard. When targeting PDF 2.0, use PDF/A-4 together with PDF/UA-2 instead (the latter is not yet supported by Typst).
- **PDF/UA-1:** The PDF/UA standard explains how to write a PDF 1.7 file optimized for Universal Access. It implies Tagged PDF, forces alternative descriptions for images and mathematics, requires a document title, and introduces rules how document contents like tables should be structured. If you are following this guide, you should be in compliance with most rules in PDF/UA-1 already. <!-- TODO Mention WTPDF? -->

When you select one or multiple of these standards for PDF export, Typst will detect if you are in violation of their rules and fail the export with a descriptive error message. You can combine multiple standards. For the strictest accessibility check currently available, choose PDF/UA-1. You can combine it with PDF/A-2a for the broadest possible range of checks. Do not disable writing tags unless you have a good reason, as they provide a baseline of accessibility across all documents you export.

Maybe you already noticed that some of the factors that go into Universal Access are hard to check automatically. For example, Typst will currently not automatically check that your color contrasts are sufficient or whether the natural language set matches the actual natural language (although the amount of spellcheck errors should provide a hint if you are using the web app). There are two international standards that address some of these human factors in more detail:

- The **[Web Content Accessibility Guidelines (WCAG)][WCAG]**: Designed by the W3C, a big international consortium behind the technologies that power the internet, WCAG describes how to make a web site accessible. All of these rules are applicable to Typst’s HTML output, and many of them apply to its PDF output.
- The **[European Norm EN 301 549][EN301549]**: Its Section 9 describes how to create accessible websites and its Section 10 describes what rules apply to non-web documents, including PDFs created by Typst. It points out which WCAG clauses are also applicable to PDFs. Conformance with this standard is a good start for complying with EU and national accessibility laws.

Keep in mind that in order to conform with EN 301 549 and the relevant WCAG provisions, your document must be tagged. If you aim for conformance, we strongly suggest using PDF/UA-1 for export to automate many of the checks for the success criteria within.

Many territories have accessibility legislation that requires you to create accessible files under some circumstances. Here are only some of them:

- **[European Accessibility Act (EAA, EU 2019/882)][EAA]**: This regulation applies to e-books, consumer banking services, e-commerce services, and more. It requires the files distributed in these applications to be accessible.
- **[Americans with Disabilities Act (ADA) Title II][ADA-2]**: This amendment of the ADA requires public sector organizations to provide files in accordance to WCAG by 2026.

Using this guide can help you reach compliance with either regulation.

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
