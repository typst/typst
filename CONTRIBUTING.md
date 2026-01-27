# Contributing to Typst
Thank you for considering to contribute to Typst. We want to foster a welcoming
and productive atmosphere for contributors. Therefore, we outline a few steps to
land your contribution below.

## What to contribute?
The most important thing about contributing is to find something that really
interests you. Contributing should be fun. While there are labels for
[good first issues][good first issue] and [larger topics suitable for
contribution][good contribution], we really recommend finding something you
personally want to see improved.

Very importantly, if you're just getting started contributing to Typst, **start
with something small.** This helps you get acquainted with the project, processes,
and design philosophy. Large architectural proposals or large PRs from new
contributors are unlikely to gain traction. Open-source development is a very
collaborative process. It fundamentally needs trust, and trust is built
step-by-step.

Before starting significant work on a feature or refactoring, please find/open
an [issue] or start a thread in the [#contributors] channel on Discord to
discuss the design. Don't be shy! Typst is a complex project with a long-term
vision and it's frustrating to find out that your idea does not align with that
vision _after_ you have already implemented something.

## How to land a contribution
1. Fork the Typst repository and start with your contribution. If you, at any
   point in this process, are unsure about how to do something in the Typst
   codebase, reach out to a maintainer or a more experienced contributor. Also
   have a look at the [`architecture.md`][architecture] file. It gives a rough
   overview over how the compiler works.
2. Create a pull request (PR) in the Typst repository, outlining your
   contribution, the **technical rationale** behind it, and, if it includes a
   new feature, **how users will use it.** Best to link to an existing issue
   with this information here. **Do not use AI to write the description.** It's
   important that you describe your ideas in your own words. It helps you with
   organizing your thoughts as much as it helps us with seeing the human thought
   process behind a particular change.
3. When you send a PR, automated CI checks will run. Your PR can only be merged
   if CI passes and **will often also only get its first review round once it
   has the green checkmark.** You can leave a comment on your PR if you need
   guidance with failing CI (or anything else).
4. A maintainer will review your PR. In this review, we check code quality,
   bugs, and whether the contribution aligns with what was previously discussed.
   If you think that a review comment misses something or is not quite right,
   please challenge it!
5. If the review passes, your PR will be merged and ship in the next version of
   Typst. You will appear as one of the contributors in the [changelog].
   Thank you!

## Signs of a good PR
Below are some signs of a good PR:
- Implements a single, self-contained feature or bugfix that has been discussed
  previously.
- Adds/changes as little code and as few interfaces as possible. Should changes
  to larger-scale abstractions be necessary, these should be discussed
  throughout the implementation process.
- Adds tests if appropriate (with reference output for visual/HTML tests). See
  the [testing] readme for more details.
- Contains documentation comments on all new Rust types.
- Comes with brief documentation for all new Typst definitions
  (elements/functions), ideally with a concise example that fits into ~5-10
  lines with <38 columns (check out existing examples for inspiration). This
  part is not too critical, as we will touch up the documentation before making
  a release.

## Review cycle
Sometimes, a contributor can become unresponsive during a review process. This
is okay! We will, however, close PRs on which we are waiting for a contributor
response after an extended period of time to avoid filling up the PR tracker
with many stale PRs. In the same way, it may take a while for us to find time to
review your PR. If there is no response after a longer while (~1 month), feel
free to ping a maintainer.

## What fits with the vision?
While Typst is an open-source project, it is also the product of a startup. We
always judge technical contributions to the project based on their technical
merits. However, as a company, our immediate priorities can and do change often
and sometimes without prior notice. This affects the design and decision making
process as well as the development and review velocity. Some proposals may also
have direct impact on our viability as a company, in which case we carefully
consider them from the business perspective.

If you are unsure whether your idea is a good fit for this project, please
discuss it with us! The core question is "Does this help to make Typst the prime
technical typesetting app?". If the answer is yes, your idea is likely right for
Typst!

[good first issue]: https://github.com/typst/typst/issues?q=state%3Aopen%20label%3A%22good%20first%20issue%22
[good contribution]: https://github.com/typst/typst/issues?q=state%3Aopen%20label%3A%22good%20contribution%22
[issue]: https://github.com/typst/typst/issues
[testing]: https://github.com/typst/typst/blob/main/tests/README.md
[#contributors]: https://discord.com/channels/1054443721975922748/1088371867913572452
[architecture]: https://github.com/typst/typst/blob/main/docs/dev/architecture.md
[changelog]: https://typst.app/docs/changelog/
