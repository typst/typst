// lined pattern
#set page(width: 500pt)
#let grad1 = gradient((10%, red), angle: 0deg)
#let grad2 = gradient((0%, red), (100%, blue), angle: 0deg)
#let grad3 = gradient((0%, red), (50%, purple), (100%, blue), angle: 0deg)


#grid(columns: (auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto, auto),
        gutter:3pt,
    ..{
        let xs = ();
        for i in range(36 * 2 + 1) {
            xs.push([
                #rect(
                fill: gradient((0%, red), (50%, purple), (100%, blue), angle: i * 5deg),
                width: 30pt, height: 30pt)
                #place(dx: 2pt, dy: -10pt, [#(i * 5)])
            ]);
        }
        xs
    }
)

// #polygon(fill: grad1, (0pt, 0pt), (100%, 0pt), (100%, 40pt))
// #polygon(fill: grad2, (0pt, 0pt), (100%, 0pt), (100%, 40pt))
// #polygon(fill: grad3, (30%, 0pt), (100%, 0pt), (100%, 40pt))
// #polygon(fill: grad3, (0pt, 0pt), (70%, 0pt), (70%, 40pt))