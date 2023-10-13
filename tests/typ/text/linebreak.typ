// Test line breaks.

---
// Test overlong word that is not directly after a hard break.
This is a spaceexceedinglylongy.

---
// Test two overlong words in a row.
Supercalifragilisticexpialidocious Expialigoricmetrioxidation.

---
// Test that there are no unwanted line break opportunities on run change.
This is partly emp#emph[has]ized.

---
Hard #linebreak() break.

---
// Test hard break directly after normal break.
Hard break directly after \ normal break.

---
// Test consecutive breaks.
Two consecutive \ \ breaks and three \ \ more.

---
// Test forcing an empty trailing line.
Trailing break \ \

---
// Test justified breaks.
#set par(justify: true)
With a soft #linebreak(justify: true)
break you can force a break without #linebreak(justify: true)
breaking justification. #linebreak(justify: false)
Nice!

---
// Test comments at the end of a line
First part//
Second part

// Test comments at the end of a line with pre-spacing
First part          //
Second part

---
// Test linebreak for East Asian languages
ทีวีตรวจทานนอร์ทแฟรีเลคเชอร์โกลด์อัลบัมเชอร์รี่เย้วสโตร์กฤษณ์เคลมเยอบีร่าพ่อค้าบลูเบอร์รี่สหัสวรรษโฮปแคนูโยโย่จูนสตรอว์เบอร์รีซื่อบื้อเยนแบ็กโฮเป็นไงโดนัททอมสเตริโอแคนูวิทย์แดรี่โดนัทวิทย์แอปพริคอทเซอร์ไพรส์ไฮบริดกิฟท์อินเตอร์โซนเซอร์วิสเทียมทานโคโยตี้ม็อบเที่ยงคืนบุญคุณ


