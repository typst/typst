// Test code highlighting with custom syntaxes.

---
#set page(width: 180pt)
#set text(6pt)

#set raw(syntaxes: (phos: "/files/Phos.sublime-syntax"))

```phos
// Performs an optical gain on an input signal,
// the maximum input power is `10dBm - gain`,
// the gain is constrained to be between 0 and 10dB.
syn gain(
    @max_power(10dBm - gain)
    input: optical,
    @range(0dB, 10dB)
    gain: Gain,
) -> @gain(gain) optical {
    ...
}
```