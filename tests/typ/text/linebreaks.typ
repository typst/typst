// Test line breaking.

---
// Test normal line breaking.
But, soft! what light through yonder window breaks? It is the east, and Juliet
is the sun. Arise, fair sun, and kill the envious moon, Who is already sick and
pale with grief hard.

// From Wikipedia:
#font("Noto Serif CJK SC")
是美国广播公司电视剧《迷失》第3季的第22和23集，也是全剧的第71集和72集
由执行制作人戴蒙·林道夫和卡尔顿·库斯编剧，导演则是另一名执行制作人杰克·本德
节目于2007年5月23日在美国和加拿大首播，共计吸引了1400万美国观众收看
本集加上插播广告一共也持续有两个小时

---
// Test hard break directly after normal break.
But, soft! What light through \ yonder window breaks?

---
// Test consecutive breaks.
But, soft! Whatlightdoyoueveryonderthrough window breaks? Why did the window
even \ break \ \ in the first place.

---
// Test two superlong words in a row.
Supercalifragilisticousalogy Expialigoricmetrioxidationreagent.

---
// Test run change after space.
Left #font("PT Sans")[Right].

---
// Test trailing newline.
Trailing newline{"\n"}
