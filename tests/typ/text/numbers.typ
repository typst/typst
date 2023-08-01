// Test how numbers are displayed

---
// Test numbers in text mode.
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

---
// Test integers.
#12 \
#1234567890 \
#0123456789 \
#0 \
#(-0) \
#(-1) \
#(-9876543210) \
#(-0987654321) \
#(4 - 8)

---
// Test floats.
#12.0 \
#3.14 \
#1234567890.0 \
#0123456789.0 \
#0.0 \
#(-0.0) \
#(-1.0) \
#(-9876543210.0) \
#(-0987654321.0) \
#(-3.14) \
#(4.0 - 8.0)
