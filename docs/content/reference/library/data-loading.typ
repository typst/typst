#import "../../../components/index.typ": docs-chapter

#show: docs-chapter.with(
  route: "/reference/library/data-loading",
)

Data loading from external files.

These functions help you with loading and embedding data, for example from the results of an experiment.

= Encoding <encoding>
Some of the functions are also capable of encoding, e.g. @cbor.encode. They facilitate passing structured data to @plugin[plugins].

However, each data format has its own native types. Therefore, for an arbitrary Typst value, the encode-to-decode roundtrip might be lossy. In general, numbers, strings, and @array[arrays] or @dictionary[dictionaries] composed of them can be reliably converted, while other types may fall back to strings via @repr, which is @repr:debugging-only[for debugging purposes only]. Please refer to the page of each data format for details.
