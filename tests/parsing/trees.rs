p "" => []
p "hi" => [T("hi")]
p "hi you" => [T("hi"), S, T("you")]
p "❤\n\n 🌍" => [T("❤"), N, T("🌍")]
p "[func]" => [F!(None)]
p "[tree][hi *you*]" => [F!(Some([T("hi"), S, B, T("you"), B]))]
// p "from [align: left] to" => [
//     T("from"), S,
//     F!("align", pos=[ID("left")], None),
//     S, T("to"),
// ]
// p "[box: x=1.2pt, false][a b c] bye" => [
//     F!(
//         "box",
//         pos=[BOOL(false)],
//         key=["x": SIZE(Size::pt(1.2))],
//         Some([T("a"), S, T("b"), S, T("c")]),
//     ),
//     S, T("bye"),
// ]
