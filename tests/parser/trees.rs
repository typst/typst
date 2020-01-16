p "" => []
p "hi" => [T("hi")]
p "hi you" => [T("hi"), S, T("you")]
p "❤\n\n 🌍" => [T("❤"), N, T("🌍")]

p "[func]" => [func!("func"; None)]
p "[tree][hi *you*]" => [func!("tree"; Some([T("hi"), S, B, T("you"), B]))]

p "from [align: left] to" => [
    T("from"), S, func!("align", pos: [ID("left")]; None), S, T("to"),
]

p "[box: x=1.2pt, false][a b c] bye" => [
    func!(
        "box",
        pos: [BOOL(false)],
        key: ["x" => SIZE(Size::pt(1.2))];
        Some([T("a"), S, T("b"), S, T("c")])
    ),
    S, T("bye"),
]

c "hi" => []
c "[align: left][\n    _body_\n]" => [
    (0:0, 0:1, B),
    (0:1, 0:6, FN),
    (0:6, 0:7, CL),
    (0:8, 0:12, ID),
    (0:12, 0:13, B),
    (0:13, 0:14, B),
    (1:4, 1:10, IT),
    (2:0, 2:2, B),
]
