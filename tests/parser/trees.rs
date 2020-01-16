// Basics.
p "" => []
p "hi" => [T("hi")]
p "hi you" => [T("hi"), S, T("you")]
p "❤\n\n 🌍" => [T("❤"), N, T("🌍")]

// Functions.
p "[func]" => [func!("func"; None)]
p "[tree][hi *you*]" => [func!("tree"; Some([T("hi"), S, B, T("you"), B]))]
p "from [align: left] to" => [
    T("from"), S, func!("align", pos: [ID("left")]; None), S, T("to"),
]
p "[f: left, 12pt, false]" => [
    func!("f", pos: [ID("left"), SIZE(Size::pt(12.0)), BOOL(false)]; None)
]
p "[f: , hi, * \"du\"]" => [func!("f", pos: [ID("hi"), STR("du")]; None)]
p "[box: x=1.2pt, false][a b c] bye" => [
    func!(
        "box",
        pos: [BOOL(false)],
        key: ["x" => SIZE(Size::pt(1.2))];
        Some([T("a"), S, T("b"), S, T("c")])
    ),
    S, T("bye"),
]

// Errors.
e "[f: , hi, * \"du\"]" => [
    (0:4,  0:5,  "expected value, found comma"),
    (0:10, 0:11, "expected value, found invalid identifier"),
]
e "[f:, , ,]" => [
    (0:3, 0:4, "expected value, found comma"),
    (0:5, 0:6, "expected value, found comma"),
    (0:7, 0:8, "expected value, found comma"),
]
e "[f:" => [(0:3, 0:3, "expected closing bracket")]
e "[f: hi" => [(0:6, 0:6, "expected closing bracket")]
e "[f: hey   12pt]" => [(0:7, 0:7, "expected comma")]
e "[box: x=, false y=z=4" => [
    (0:8,  0:9,  "expected value, found comma"),
    (0:15, 0:15, "expected comma"),
    (0:19, 0:19, "expected comma"),
    (0:19, 0:20, "expected value, found equals sign"),
    (0:21, 0:21, "expected closing bracket"),
]
