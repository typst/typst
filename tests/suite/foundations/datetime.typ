--- datetime-constructor-empty paged ---
// Error: 2-12 at least one of date or time must be fully specified
// Hint: 2-12 add the `hour`, `minute`, and `second` arguments to get a valid time
// Hint: 2-12 add the `year`, `month`, and `day` arguments to get a valid date
#datetime()

--- datetime-constructor-time-invalid paged ---
// Error: 2-42 time is invalid
#datetime(hour: 25, minute: 0, second: 0)

--- datetime-constructor-date-invalid paged ---
// Error: 2-41 date is invalid
#datetime(year: 2000, month: 2, day: 30)

--- datetime-display paged ---
// Test displaying of dates.
#test(datetime(year: 2023, month: 4, day: 29).display(), "2023-04-29")
#test(datetime(year: 2023, month: 4, day: 29).display("[year]"), "2023")
#test(
  datetime(year: 2023, month: 4, day: 29)
    .display("[year repr:last_two]"),
  "23",
)
#test(
  datetime(year: 2023, month: 4, day: 29)
    .display("[year] [month repr:long] [day] [week_number] [weekday]"),
  "2023 April 29 17 Saturday",
)

// Test displaying of times
#test(datetime(hour: 14, minute: 26, second: 50).display(), "14:26:50")
#test(datetime(hour: 14, minute: 26, second: 50).display("[hour]"), "14")
#test(
  datetime(hour: 14, minute: 26, second: 50)
    .display("[hour repr:12 padding:none]"),
  "2",
)
#test(
  datetime(hour: 14, minute: 26, second: 50)
    .display("[hour], [minute], [second]"), "14, 26, 50",
)

// Test displaying of datetimes
#test(
  datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50).display(),
  "2023-04-29 14:26:50",
)

// Test getting the year/month/day etc. of a datetime
#let d = datetime(year: 2023, month: 4, day: 29, hour: 14, minute: 26, second: 50)
#test(d.year(), 2023)
#test(d.month(), 4)
#test(d.weekday(), 6)
#test(d.day(), 29)
#test(d.hour(), 14)
#test(d.minute(), 26)
#test(d.second(), 50)

#let e = datetime(year: 2023, month: 4, day: 29)
#test(e.hour(), none)
#test(e.minute(), none)
#test(e.second(), none)

// Test today
#test(datetime.today().display(), "1970-01-01")
#test(datetime.today(offset: auto).display(), "1970-01-01")
#test(datetime.today(offset: 2).display(), "1970-01-01")

--- datetime-ordinal paged ---
// Test date methods.
#test(datetime(day: 1, month: 1, year: 2000).ordinal(), 1);
#test(datetime(day: 1, month: 3, year: 2000).ordinal(), 31 + 29 + 1);
#test(datetime(day: 31, month: 12, year: 2000).ordinal(), 366);
#test(datetime(day: 1, month: 3, year: 2001).ordinal(), 31 + 28 + 1);
#test(datetime(day: 31, month: 12, year: 2001).ordinal(), 365);

--- datetime-incomplete-time-1 paged ---
// Error: 2-34 time is incomplete
// Hint: 2-34 add the `hour` argument to get a valid time
#datetime(minute: 14, second: 30)

--- datetime-incomplete-time-2 paged ---
// Error: 2-20 time is incomplete
// Hint: 2-20 add the `minute` and `second` arguments to get a valid time
#datetime(hour: 14)

--- datetime-incomplete-date-1 paged ---
// Error: 2-31 date is incomplete
// Hint: 2-31 add the `month` argument to get a valid date
#datetime(year: 2014, day: 30)

--- datetime-incomplete-date-2 paged ---
// Error: 2-20 date is incomplete
// Hint: 2-20 add the `year` and `day` arguments to get a valid date
#datetime(month: 5)

--- datetime-display-missing-closing-bracket paged ---
// Error: 27-34 missing closing bracket for bracket at index 0
#datetime.today().display("[year")

--- datetime-display-invalid-component paged ---
// Error: 27-38 invalid component name 'nothing' at index 1
#datetime.today().display("[nothing]")

--- datetime-display-invalid-modifier paged ---
// Error: 27-50 invalid modifier 'wrong' at index 6
#datetime.today().display("[year wrong:last_two]")

--- datetime-display-expected-component paged ---
// Error: 27-33 expected component name at index 2
#datetime.today().display("  []")

--- datetime-display-insufficient-information paged ---
// Error: 2-36 failed to format datetime (insufficient information)
#datetime.today().display("[hour]")
