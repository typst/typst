// Test using the `grid` function to create a finance table.

// ---
// #page(width: 12cm, height: 2.5cm)
// #grid(
//     columns: 5,
//     gutter-columns: (2fr, 1fr, 1fr),
//     gutter-rows: 4 * (6pt,),
//     [*Quarter*],
//     [Expenditure],
//     [External Revenue],
//     [Financial ROI],
//     [_total_],
//     [*Q1*],
//     [173,472.57 \$],
//     [472,860.91 \$],
//     [51,286.84 \$],
//     [_350,675.18 \$_],
//     [*Q2*],
//     [93,382.12 \$],
//     [439,382.85 \$],
//     [-1,134.30 \$],
//     [_344,866.43 \$_],
//     [*Q3*],
//     [96,421.49 \$],
//     [238,583.54 \$],
//     [3,497.12 \$],
//     [_145,659.17 \$_],
// )

// ---
#page(width: 5cm, height: 5cm)
#grid(
    columns: 2,
    [Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Donec quam felis, ultricies nec, pellentesque eu, pretium quis, sem. Nulla consequat massa quis enim. Donec pede justo, fringilla vel, aliquet nec, vulputate eget, arcu.

In enim justo, rhoncus ut, imperdiet a, venenatis vitae, justo. Nullam dictum felis eu pede mollis pretium. Integer tincidunt. Cras dapibus. Vivamus elementum semper nisi. Aenean vulputate eleifend tellus. Aenean leo ligula, porttitor eu, consequat vitae, eleifend ac, enim. Aliquam lorem ante, dapibus in, viverra quis, feugiat a, tellus.],
    [Text that is rather short],
    [Another column],
    [And another column],
)
