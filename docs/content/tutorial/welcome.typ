#import "../../components/index.typ": docs-chapter

#show: docs-chapter.with(
  title: "Tutorial",
  route: "/tutorial/welcome",
  description: "Typst's tutorial.",
)

Welcome to Typst's tutorial! In this tutorial, you will learn how to write and format documents in Typst. We will start with everyday tasks and gradually introduce more advanced features. This tutorial does not assume prior knowledge of Typst, other markup languages, or programming. We do assume that you know how to edit a text file.

The best way to start is to sign up to the Typst app for free and follow along with the steps below. The app gives you instant preview, syntax highlighting and helpful autocompletions. Alternatively, you can follow along in your local text editor with the #link("https://github.com/typst/typst")[open-source CLI].

= When to use Typst <when-typst>
Before we get started, let's check what Typst is and when to use it. Typst is a markup language for typesetting documents. It is designed to be easy to learn, fast, and versatile. Typst takes text files with markup in them and outputs PDFs.

Typst is a good choice for writing any long form text such as essays, articles, scientific papers, books, reports, and homework assignments. Moreover, Typst is a great fit for any documents containing mathematical notation, such as papers in the math, physics, and engineering fields. Finally, due to its strong styling and automation features, it is an excellent choice for any set of documents that share a common style, such as a book series.

= What you will learn <learnings>
This tutorial has four chapters. Each chapter builds on the previous one. Here is what you will learn in each of them:

+ @tutorial:writing-in-typst[Writing in Typst:] Learn how to write text and insert images, equations, and other elements.
+ @tutorial:formatting[Formatting:] Learn how to adjust the formatting of your document, including font size, heading styles, and more.
+ @tutorial:advanced-styling[Advanced Styling:] Create a complex page layout for a scientific paper with typographic features such as an author list and run-in headings.
+ @tutorial:making-a-template[Making a Template:] Build a reusable template from the paper you created in the previous chapter.

We hope you'll enjoy Typst!
