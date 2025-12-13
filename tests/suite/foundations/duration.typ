// Test durations.

--- duration-negate paged ---
// Test negating durations.
#test(-duration(hours: 2), duration(hours: -2))

--- duration-add-and-subtract paged ---
// Test adding and subtracting durations.
#test(duration(weeks: 1, hours: 1), duration(weeks: 1) + duration(hours: 1))
#test(duration(weeks: 1, hours: -1), duration(weeks: 1) - duration(hours: 1))
#test(duration(days: 6, hours: 23), duration(weeks: 1) - duration(hours: 1))

--- duration-add-and-subtract-dates paged ---
// Test adding and subtracting durations and dates.
#let d = datetime(day: 1, month: 1, year: 2000)
#let d2 = datetime(day: 1, month: 2, year: 2000)
#test(d + duration(weeks: 2), datetime(day: 15, month: 1, year: 2000))
#test(d + duration(days: 3), datetime(day: 4, month: 1, year: 2000))
#test(d + duration(weeks: 1, days: 3), datetime(day: 11, month: 1, year: 2000))
#test(d2 + duration(days: -1), datetime(day: 31, month: 1, year: 2000))
#test(d2 + duration(days: -3), datetime(day: 29, month: 1, year: 2000))
#test(d2 + duration(weeks: -1), datetime(day: 25, month: 1, year: 2000))
#test(d + duration(days: -1), datetime(day: 31, month: 12, year: 1999))
#test(d + duration(weeks: 1, days: -7), datetime(day: 1, month: 1, year: 2000))
#test(d2 - duration(days: 1), datetime(day: 31, month: 1, year: 2000))
#test(d2 - duration(days: 3), datetime(day: 29, month: 1, year: 2000))
#test(d2 - duration(weeks: 1), datetime(day: 25, month: 1, year: 2000))
#test(d - duration(days: 1), datetime(day: 31, month: 12, year: 1999))
#test(datetime(day: 31, month: 1, year: 2000) + duration(days: 1), d2)
#test(
  datetime(day: 31, month: 12, year: 2000) + duration(days: 1),
  datetime(day: 1, month: 1, year: 2001),
)

--- duration-add-and-subtract-times paged ---
// Test adding and subtracting durations and times.
#let a = datetime(hour: 12, minute: 0, second: 0)
#test(a + duration(hours: 1, minutes: -60), datetime(hour: 12, minute: 0, second: 0))
#test(a + duration(hours: 2), datetime(hour: 14, minute: 0, second: 0))
#test(a + duration(minutes: 10), datetime(hour: 12, minute: 10, second: 0))
#test(a + duration(seconds: 30), datetime(hour: 12, minute: 0, second: 30))
#test(a + duration(hours: -2), datetime(hour: 10, minute: 0, second: 0))
#test(a - duration(hours: 2), datetime(hour: 10, minute: 0, second: 0))
#test(a + duration(minutes: -10), datetime(hour: 11, minute: 50, second: 0))
#test(a - duration(minutes: 10), datetime(hour: 11, minute: 50, second: 0))
#test(a + duration(seconds: -30), datetime(hour: 11, minute: 59, second: 30))
#test(a - duration(seconds: 30), datetime(hour: 11, minute: 59, second: 30))
#test(
  a + duration(hours: 1, minutes: 13, seconds: 13),
  datetime(hour: 13, minute: 13, second: 13),
)

--- duration-add-and-subtract-datetimes paged ---
// Test adding and subtracting durations and datetimes.
#test(
  datetime(day: 1, month: 1, year: 2000, hour: 12, minute: 0, second: 0)
    + duration(weeks: 1, days: 3, hours: -13, minutes: 10, seconds: -10 ),
  datetime(day: 10, month: 1, year: 2000, hour: 23, minute: 9, second: 50),
)
#test(
  datetime(day: 1, month: 1, year: 2000, hour: 12, minute: 0, second: 0)
    + duration(weeks: 1, days: 3, minutes: 10)
    - duration(hours: 13, seconds: 10),
  datetime(day: 10, month: 1, year: 2000, hour: 23, minute: 9, second: 50),
)

--- duration-from-date-subtraction paged ---
// Test subtracting dates.
#let a = datetime(hour: 12, minute: 0, second: 0)
#let b = datetime(day: 1, month: 1, year: 2000)
#test(datetime(hour: 14, minute: 0, second: 0) - a, duration(hours: 2))
#test(datetime(hour: 14, minute: 0, second: 0) - a, duration(minutes: 120))
#test(datetime(hour: 13, minute: 0, second: 0) - a, duration(seconds: 3600))
#test(datetime(day: 1, month: 2, year: 2000) - b, duration(days: 31))
#test(datetime(day: 15, month: 1, year: 2000) - b, duration(weeks: 2))

--- duration-multiply-with-number paged ---
// Test multiplying and dividing durations with numbers.
#test(duration(minutes: 10) * 6, duration(hours: 1))
#test(duration(minutes: 10) * 2, duration(minutes: 20))
#test(duration(minutes: 10) * 2.5, duration(minutes: 25))
#test(duration(minutes: 10) / 2, duration(minutes: 5))
#test(duration(minutes: 10) / 2.5, duration(minutes: 4))

--- duration-divide paged ---
// Test dividing durations with durations
#test(duration(minutes: 20) / duration(hours: 1), 1 / 3)
#test(duration(minutes: 20) / duration(minutes: 10), 2)
#test(duration(minutes: 20) / duration(minutes: 8), 2.5)

--- duration-compare paged ---
// Test comparing durations
#test(duration(minutes: 20) > duration(minutes: 10), true)
#test(duration(minutes: 20) >= duration(minutes: 10), true)
#test(duration(minutes: 10) < duration(minutes: 20), true)
#test(duration(minutes: 10) <= duration(minutes: 20), true)
#test(duration(minutes: 10) == duration(minutes: 10), true)
#test(duration(minutes: 10) != duration(minutes: 20), true)
#test(duration(minutes: 10) <= duration(minutes: 10), true)
#test(duration(minutes: 10) >= duration(minutes: 10), true)
#test(duration(minutes: 20) < duration(minutes: 10), false)
#test(duration(minutes: 20) <= duration(minutes: 10), false)
#test(duration(minutes: 20) == duration(minutes: 10), false)
