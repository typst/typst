// Test how numbers are displayed.

--- numbers paged ---
// Test numbers in text mode.
#set text(font: ("Libertinus Serif", "Noto Sans Arabic"))
12 \
12.0 \
3.14 \
1234567890 \
0123456789 \
0 \
0.0 \
+0 \
+0.0 \
-0 \
-0.0 \
-1 \
-3.14 \
-9876543210 \
-0987654321 \
٣٫١٤ \
-٣٫١٤ \
-¾ \
#text(fractions: true)[-3/2] \
2022 - 2023 \
2022 -- 2023 \
2022--2023 \
2022-2023 \
٢٠٢٢ - ٢٠٢٣ \
٢٠٢٢ -- ٢٠٢٣ \
٢٠٢٢--٢٠٢٣ \
٢٠٢٢-٢٠٢٣ \
-500 -- -400
